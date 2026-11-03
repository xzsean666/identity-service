use crate::db::{
    create_user_with_identity, find_identities_by_user_id, find_identity, find_user_by_id,
    link_identity_to_user,
};
use crate::error::AppError;
use crate::models::{ExternalIdentity, Identity, User};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct IdentityService {
    pool: PgPool,
}

impl IdentityService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn resolve_or_create_user(
        &self,
        ext: &ExternalIdentity,
    ) -> Result<User, AppError> {
        // 1. Check if identity already exists
        if let Some(existing_identity) =
            find_identity(&self.pool, &ext.provider, &ext.provider_user_id).await?
        {
            let user = find_user_by_id(&self.pool, existing_identity.user_id)
                .await?
                .ok_or_else(|| {
                    AppError::Internal(format!(
                        "Database consistency error: identity exists for user_id {} but user record was not found",
                        existing_identity.user_id
                    ))
                })?;

            if user.status != "active" {
                return Err(AppError::Forbidden(format!(
                    "User account is {} (not active)",
                    user.status
                )));
            }

            return Ok(user);
        }

        // 2. Atomically create user and identity
        let (user, _identity) = create_user_with_identity(&self.pool, ext).await?;
        tracing::info!(
            user_id = %user.id,
            provider = %ext.provider,
            "Created new user from external identity"
        );

        Ok(user)
    }

    pub async fn link_identity(
        &self,
        user_id: Uuid,
        ext: &ExternalIdentity,
    ) -> Result<Identity, AppError> {
        // Verify target user exists and is active
        let user = find_user_by_id(&self.pool, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User not found: {user_id}")))?;

        if user.status != "active" {
            return Err(AppError::Forbidden(format!(
                "Cannot link identity: user account is {}",
                user.status
            )));
        }

        link_identity_to_user(&self.pool, user_id, ext).await
    }

    pub async fn get_user_identities(&self, user_id: Uuid) -> Result<Vec<Identity>, AppError> {
        find_identities_by_user_id(&self.pool, user_id).await
    }
}
