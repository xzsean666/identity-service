use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::{
    application::{
        auth::LocalRegistrationRepository, error::AppError, identity_binding::IdentityRepository,
    },
    domain::{
        identity::{ExternalIdentity, NormalizedExternalIdentity},
        user::InternalUser,
    },
    providers::local_password::LocalCredential,
};

use super::{
    account_status_to_database, datetime_to_database, local_credential_status_to_database,
    map_sqlx_error, row_to_internal_user, uuid_to_database,
};

#[derive(Clone)]
pub struct SqliteIdentityRepository {
    pool: SqlitePool,
}

impl SqliteIdentityRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IdentityRepository for SqliteIdentityRepository {
    async fn insert_active_user(&self, user: InternalUser) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO internal_users (
                internal_user_id,
                account_status,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4)
            "#,
        )
        .bind(uuid_to_database(user.internal_user_id))
        .bind(account_status_to_database(&user.account_status))
        .bind(datetime_to_database(user.created_at))
        .bind(datetime_to_database(user.updated_at))
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn bound_user(
        &self,
        external_identity: &NormalizedExternalIdentity,
    ) -> Result<Option<InternalUser>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT
                internal_users.internal_user_id,
                internal_users.account_status,
                internal_users.created_at,
                internal_users.updated_at
            FROM external_identities
            JOIN internal_users
                ON internal_users.internal_user_id = external_identities.internal_user_id
            WHERE external_identities.provider_name = ?1
                AND external_identities.provider_subject = ?2
            "#,
        )
        .bind(&external_identity.provider_name)
        .bind(&external_identity.provider_subject)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        row.map(row_to_internal_user).transpose()
    }

    async fn bind_new_active_user(
        &self,
        external_identity: NormalizedExternalIdentity,
        now: DateTime<Utc>,
    ) -> Result<InternalUser, AppError> {
        let user = InternalUser::new_active(now);
        let binding = ExternalIdentity {
            provider_name: external_identity.provider_name,
            provider_subject: external_identity.provider_subject,
            internal_user_id: user.internal_user_id,
            provider_metadata: external_identity.provider_metadata,
            created_at: now,
            updated_at: now,
        };
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;

        sqlx::query(
            r#"
            INSERT INTO internal_users (
                internal_user_id,
                account_status,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4)
            "#,
        )
        .bind(uuid_to_database(user.internal_user_id))
        .bind(account_status_to_database(&user.account_status))
        .bind(datetime_to_database(user.created_at))
        .bind(datetime_to_database(user.updated_at))
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        sqlx::query(
            r#"
            INSERT INTO external_identities (
                provider_name,
                provider_subject,
                internal_user_id,
                provider_metadata,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(&binding.provider_name)
        .bind(&binding.provider_subject)
        .bind(uuid_to_database(binding.internal_user_id))
        .bind(binding.provider_metadata.to_string())
        .bind(datetime_to_database(binding.created_at))
        .bind(datetime_to_database(binding.updated_at))
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(user)
    }

    async fn bind_existing_user(
        &self,
        internal_user_id: Uuid,
        external_identity: NormalizedExternalIdentity,
        now: DateTime<Utc>,
    ) -> Result<InternalUser, AppError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        let user_row = sqlx::query(
            r#"
            SELECT internal_user_id, account_status, created_at, updated_at
            FROM internal_users
            WHERE internal_user_id = ?1
            "#,
        )
        .bind(uuid_to_database(internal_user_id))
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let user = user_row
            .map(row_to_internal_user)
            .transpose()?
            .ok_or(AppError::Unauthorized)?;

        let existing_binding = sqlx::query(
            r#"
            SELECT internal_user_id
            FROM external_identities
            WHERE provider_name = ?1 AND provider_subject = ?2
            "#,
        )
        .bind(&external_identity.provider_name)
        .bind(&external_identity.provider_subject)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        if let Some(row) = existing_binding {
            let existing_user_id: String =
                row.try_get("internal_user_id").map_err(map_sqlx_error)?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            if existing_user_id == internal_user_id.to_string() {
                return Ok(user);
            }
            return Err(AppError::IdentityConflict);
        }

        sqlx::query(
            r#"
            INSERT INTO external_identities (
                provider_name,
                provider_subject,
                internal_user_id,
                provider_metadata,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(&external_identity.provider_name)
        .bind(&external_identity.provider_subject)
        .bind(uuid_to_database(internal_user_id))
        .bind(external_identity.provider_metadata.to_string())
        .bind(datetime_to_database(now))
        .bind(datetime_to_database(now))
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(user)
    }

    async fn user_by_id(&self, internal_user_id: Uuid) -> Result<InternalUser, AppError> {
        let row = sqlx::query(
            r#"
            SELECT internal_user_id, account_status, created_at, updated_at
            FROM internal_users
            WHERE internal_user_id = ?1
            "#,
        )
        .bind(uuid_to_database(internal_user_id))
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        row.map(row_to_internal_user)
            .transpose()?
            .ok_or(AppError::Unauthorized)
    }

    async fn delete_user(&self, internal_user_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            DELETE FROM internal_users
            WHERE internal_user_id = ?1
            "#,
        )
        .bind(uuid_to_database(internal_user_id))
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }
}

#[async_trait]
impl LocalRegistrationRepository for SqliteIdentityRepository {
    async fn register_local_user(
        &self,
        user: InternalUser,
        credential: LocalCredential,
        external_identity: NormalizedExternalIdentity,
    ) -> Result<InternalUser, AppError> {
        let binding = ExternalIdentity {
            provider_name: external_identity.provider_name,
            provider_subject: external_identity.provider_subject,
            internal_user_id: user.internal_user_id,
            provider_metadata: external_identity.provider_metadata,
            created_at: user.created_at,
            updated_at: user.updated_at,
        };
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;

        sqlx::query(
            r#"
            INSERT INTO internal_users (
                internal_user_id,
                account_status,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4)
            "#,
        )
        .bind(uuid_to_database(user.internal_user_id))
        .bind(account_status_to_database(&user.account_status))
        .bind(datetime_to_database(user.created_at))
        .bind(datetime_to_database(user.updated_at))
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

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
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
        )
        .bind(uuid_to_database(credential.credential_id))
        .bind(uuid_to_database(credential.internal_user_id))
        .bind(&credential.username)
        .bind(&credential.normalized_username)
        .bind(&credential.password_hash)
        .bind(&credential.password_hash_algorithm)
        .bind(&credential.password_hash_parameters)
        .bind(local_credential_status_to_database(&credential.status))
        .bind(datetime_to_database(credential.created_at))
        .bind(datetime_to_database(credential.updated_at))
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        sqlx::query(
            r#"
            INSERT INTO external_identities (
                provider_name,
                provider_subject,
                internal_user_id,
                provider_metadata,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
        )
        .bind(&binding.provider_name)
        .bind(&binding.provider_subject)
        .bind(uuid_to_database(binding.internal_user_id))
        .bind(binding.provider_metadata.to_string())
        .bind(datetime_to_database(binding.created_at))
        .bind(datetime_to_database(binding.updated_at))
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;

        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(user)
    }
}
