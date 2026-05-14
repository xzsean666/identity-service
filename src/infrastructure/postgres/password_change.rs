use async_trait::async_trait;
use chrono::Duration;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{
        error::AppError,
        password_change::{PasswordChangeCommand, PasswordChangeRepository},
    },
    domain::session::{RefreshTokenRecord, RefreshTokenStatus, SessionStatus},
    providers::local_password::LocalCredentialStatus,
};

use super::{
    local_credential_status_to_database, map_sqlx_error, row_to_session,
    session::insert_refresh_token,
};

#[derive(Clone)]
pub struct PostgresPasswordChangeRepository {
    pool: PgPool,
}

impl PostgresPasswordChangeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PasswordChangeRepository for PostgresPasswordChangeRepository {
    async fn change_password_and_rotate_refresh_tokens(
        &self,
        command: PasswordChangeCommand,
    ) -> Result<RefreshTokenRecord, AppError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        let session_row = sqlx::query(
            r#"
            SELECT
                session_id,
                internal_user_id,
                provider_name,
                client_id,
                device_metadata,
                status,
                issued_at,
                expires_at,
                revoked_at
            FROM sessions
            WHERE session_id = $1
            FOR UPDATE
            "#,
        )
        .bind(command.current_session_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let session = session_row
            .map(row_to_session)
            .transpose()?
            .ok_or(AppError::Unauthorized)?;

        if session.internal_user_id != command.internal_user_id
            || session.status != SessionStatus::Active
            || session.expires_at <= command.now
        {
            return Err(AppError::Unauthorized);
        }

        let prepared_password_change = command.prepared_password_change;
        if prepared_password_change.credential.status != LocalCredentialStatus::Active {
            return Err(AppError::InvalidCredentials);
        }
        let previous_password_hash = prepared_password_change.previous_password_hash;
        let credential = prepared_password_change.credential;
        let update_result = sqlx::query(
            r#"
            UPDATE local_credentials
            SET
                username = $3,
                normalized_username = $4,
                password_hash = $5,
                password_hash_algorithm = $6,
                password_hash_parameters = $7,
                status = $8,
                updated_at = $9
            WHERE internal_user_id = $1
                AND password_hash = $2
                AND status = 'active'
            "#,
        )
        .bind(command.internal_user_id)
        .bind(&previous_password_hash)
        .bind(&credential.username)
        .bind(&credential.normalized_username)
        .bind(&credential.password_hash)
        .bind(&credential.password_hash_algorithm)
        .bind(&credential.password_hash_parameters)
        .bind(local_credential_status_to_database(&credential.status))
        .bind(credential.updated_at)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        if update_result.rows_affected() == 0 {
            return Err(AppError::InvalidCredentials);
        }

        sqlx::query(
            r#"
            UPDATE refresh_token_records
            SET status = 'revoked', revoked_at = $2
            WHERE internal_user_id = $1
            "#,
        )
        .bind(command.internal_user_id)
        .bind(command.now)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        let new_record = RefreshTokenRecord {
            refresh_token_id: Uuid::new_v4(),
            session_id: command.current_session_id,
            internal_user_id: command.internal_user_id,
            token_family_id: Uuid::new_v4(),
            token_hash: command.new_token_hash,
            status: RefreshTokenStatus::Active,
            issued_at: command.now,
            expires_at: command.now + Duration::seconds(command.refresh_token_lifetime_seconds),
            consumed_at: None,
            revoked_at: None,
        };
        insert_refresh_token(&mut transaction, &new_record).await?;

        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(new_record)
    }
}
