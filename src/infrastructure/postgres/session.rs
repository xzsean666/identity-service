use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::{error::AppError, session::SessionRepository},
    domain::session::{RefreshTokenRecord, RefreshTokenStatus, Session, SessionStatus},
};

use super::{
    map_sqlx_error, refresh_token_status_to_database, row_to_refresh_token_record, row_to_session,
    session_status_to_database,
};

#[derive(Clone)]
pub struct PostgresSessionRepository {
    pool: PgPool,
}

impl PostgresSessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SessionRepository for PostgresSessionRepository {
    async fn create_session_with_refresh(
        &self,
        session: Session,
        refresh_token: RefreshTokenRecord,
    ) -> Result<(Session, RefreshTokenRecord), AppError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;

        insert_session(&mut transaction, &session).await?;
        insert_refresh_token(&mut transaction, &refresh_token).await?;

        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok((session, refresh_token))
    }

    async fn exchange_refresh(
        &self,
        token_hash: &str,
        next_token_hash: String,
        refresh_token_lifetime_seconds: i64,
        now: DateTime<Utc>,
    ) -> Result<(Session, RefreshTokenRecord), AppError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        let existing_row = sqlx::query(
            r#"
            SELECT
                refresh_token_id,
                session_id,
                internal_user_id,
                token_family_id,
                token_hash,
                status,
                issued_at,
                expires_at,
                consumed_at,
                revoked_at
            FROM refresh_token_records
            WHERE token_hash = $1
            FOR UPDATE
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        let existing = existing_row
            .map(row_to_refresh_token_record)
            .transpose()?
            .ok_or(AppError::TokenInvalid)?;

        if existing.status == RefreshTokenStatus::Consumed {
            sqlx::query(
                r#"
                UPDATE refresh_token_records
                SET
                    status = CASE
                        WHEN refresh_token_id = $1 THEN 'reused'
                        ELSE 'revoked'
                    END,
                    revoked_at = $2
                WHERE token_family_id = $3
                "#,
            )
            .bind(existing.refresh_token_id)
            .bind(now)
            .bind(existing.token_family_id)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Err(AppError::RefreshTokenReused);
        }

        if existing.status != RefreshTokenStatus::Active {
            return Err(AppError::TokenInvalid);
        }

        if existing.expires_at <= now {
            sqlx::query(
                r#"
                UPDATE refresh_token_records
                SET status = 'expired'
                WHERE refresh_token_id = $1
                "#,
            )
            .bind(existing.refresh_token_id)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Err(AppError::TokenInvalid);
        }

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
        .bind(existing.session_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let session = session_row
            .map(row_to_session)
            .transpose()?
            .ok_or(AppError::TokenInvalid)?;

        if session.status != SessionStatus::Active || session.expires_at <= now {
            return Err(AppError::TokenInvalid);
        }

        sqlx::query(
            r#"
            UPDATE refresh_token_records
            SET status = 'consumed', consumed_at = $2
            WHERE refresh_token_id = $1
            "#,
        )
        .bind(existing.refresh_token_id)
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        let new_record = RefreshTokenRecord {
            refresh_token_id: Uuid::new_v4(),
            session_id: existing.session_id,
            internal_user_id: existing.internal_user_id,
            token_family_id: existing.token_family_id,
            token_hash: next_token_hash,
            status: RefreshTokenStatus::Active,
            issued_at: now,
            expires_at: now + Duration::seconds(refresh_token_lifetime_seconds),
            consumed_at: None,
            revoked_at: None,
        };
        insert_refresh_token(&mut transaction, &new_record).await?;

        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok((session, new_record))
    }

    async fn revoke_session(&self, session_id: Uuid, now: DateTime<Utc>) -> Result<(), AppError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        let result = sqlx::query(
            r#"
            UPDATE sessions
            SET status = 'revoked', revoked_at = $2
            WHERE session_id = $1
            "#,
        )
        .bind(session_id)
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(AppError::Unauthorized);
        }

        sqlx::query(
            r#"
            UPDATE refresh_token_records
            SET status = 'revoked', revoked_at = $2
            WHERE session_id = $1
            "#,
        )
        .bind(session_id)
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(())
    }

    async fn rotate_all_user_refresh_families(
        &self,
        internal_user_id: Uuid,
        current_session_id: Uuid,
        new_token_hash: String,
        refresh_token_lifetime_seconds: i64,
        now: DateTime<Utc>,
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
        .bind(current_session_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let session = session_row
            .map(row_to_session)
            .transpose()?
            .ok_or(AppError::Unauthorized)?;

        if session.internal_user_id != internal_user_id
            || session.status != SessionStatus::Active
            || session.expires_at <= now
        {
            return Err(AppError::Unauthorized);
        }

        sqlx::query(
            r#"
            UPDATE refresh_token_records
            SET status = 'revoked', revoked_at = $2
            WHERE internal_user_id = $1
            "#,
        )
        .bind(internal_user_id)
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        let new_record = RefreshTokenRecord {
            refresh_token_id: Uuid::new_v4(),
            session_id: current_session_id,
            internal_user_id,
            token_family_id: Uuid::new_v4(),
            token_hash: new_token_hash,
            status: RefreshTokenStatus::Active,
            issued_at: now,
            expires_at: now + Duration::seconds(refresh_token_lifetime_seconds),
            consumed_at: None,
            revoked_at: None,
        };
        insert_refresh_token(&mut transaction, &new_record).await?;

        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(new_record)
    }

    async fn active_session_by_id(
        &self,
        session_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Session, AppError> {
        let row = sqlx::query(
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
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;
        let session = row
            .map(row_to_session)
            .transpose()?
            .ok_or(AppError::Unauthorized)?;

        if session.status != SessionStatus::Active || session.expires_at <= now {
            return Err(AppError::Unauthorized);
        }

        Ok(session)
    }
}

async fn insert_session(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    session: &Session,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO sessions (
            session_id,
            internal_user_id,
            provider_name,
            client_id,
            device_metadata,
            status,
            issued_at,
            expires_at,
            revoked_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(session.session_id)
    .bind(session.internal_user_id)
    .bind(&session.provider_name)
    .bind(&session.client_id)
    .bind(&session.device_metadata)
    .bind(session_status_to_database(&session.status))
    .bind(session.issued_at)
    .bind(session.expires_at)
    .bind(session.revoked_at)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;

    Ok(())
}

pub(crate) async fn insert_refresh_token(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    refresh_token: &RefreshTokenRecord,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO refresh_token_records (
            refresh_token_id,
            session_id,
            internal_user_id,
            token_family_id,
            token_hash,
            status,
            issued_at,
            expires_at,
            consumed_at,
            revoked_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
    )
    .bind(refresh_token.refresh_token_id)
    .bind(refresh_token.session_id)
    .bind(refresh_token.internal_user_id)
    .bind(refresh_token.token_family_id)
    .bind(&refresh_token.token_hash)
    .bind(refresh_token_status_to_database(&refresh_token.status))
    .bind(refresh_token.issued_at)
    .bind(refresh_token.expires_at)
    .bind(refresh_token.consumed_at)
    .bind(refresh_token.revoked_at)
    .execute(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;

    Ok(())
}
