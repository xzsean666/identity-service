use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    application::error::AppError,
    providers::local_password::{LocalCredential, LocalCredentialRepository},
};

use super::{local_credential_status_to_database, map_sqlx_error, row_to_local_credential};

#[derive(Clone)]
pub struct PostgresLocalCredentialRepository {
    pool: PgPool,
}

impl PostgresLocalCredentialRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LocalCredentialRepository for PostgresLocalCredentialRepository {
    async fn create_credential(
        &self,
        normalized_username: &str,
        credential: LocalCredential,
    ) -> Result<LocalCredential, AppError> {
        sqlx::query(
            r#"
            INSERT INTO local_credentials (
                credential_id,
                internal_user_id,
                username,
                normalized_username,
                password_hash,
                password_hash_algorithm,
                password_hash_parameters,
                status,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(credential.credential_id)
        .bind(credential.internal_user_id)
        .bind(&credential.username)
        .bind(normalized_username)
        .bind(&credential.password_hash)
        .bind(&credential.password_hash_algorithm)
        .bind(&credential.password_hash_parameters)
        .bind(local_credential_status_to_database(&credential.status))
        .bind(credential.created_at)
        .bind(credential.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(credential)
    }

    async fn find_by_normalized_username(
        &self,
        normalized_username: &str,
    ) -> Result<Option<LocalCredential>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT
                credential_id,
                internal_user_id,
                username,
                normalized_username,
                password_hash,
                password_hash_algorithm,
                password_hash_parameters,
                status,
                created_at,
                updated_at
            FROM local_credentials
            WHERE normalized_username = $1
            "#,
        )
        .bind(normalized_username)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        row.map(row_to_local_credential).transpose()
    }

    async fn find_by_internal_user_id(
        &self,
        internal_user_id: Uuid,
    ) -> Result<Option<LocalCredential>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT
                credential_id,
                internal_user_id,
                username,
                normalized_username,
                password_hash,
                password_hash_algorithm,
                password_hash_parameters,
                status,
                created_at,
                updated_at
            FROM local_credentials
            WHERE internal_user_id = $1
            "#,
        )
        .bind(internal_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        row.map(row_to_local_credential).transpose()
    }

    async fn update_for_internal_user_id(
        &self,
        internal_user_id: Uuid,
        credential: LocalCredential,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            UPDATE local_credentials
            SET
                username = $2,
                normalized_username = $3,
                password_hash = $4,
                password_hash_algorithm = $5,
                password_hash_parameters = $6,
                status = $7,
                updated_at = $8
            WHERE internal_user_id = $1
            "#,
        )
        .bind(internal_user_id)
        .bind(&credential.username)
        .bind(&credential.normalized_username)
        .bind(&credential.password_hash)
        .bind(&credential.password_hash_algorithm)
        .bind(&credential.password_hash_parameters)
        .bind(local_credential_status_to_database(&credential.status))
        .bind(credential.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(AppError::InvalidCredentials);
        }

        Ok(())
    }
}
