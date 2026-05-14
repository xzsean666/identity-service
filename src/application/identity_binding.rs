use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use async_trait::async_trait;

use crate::{
    application::error::AppError,
    domain::{
        identity::{BindingMode, NormalizedExternalIdentity},
        user::{AccountStatus, InternalUser},
    },
};

#[async_trait]
pub trait IdentityRepository: Send + Sync {
    async fn insert_active_user(&self, user: InternalUser) -> Result<(), AppError>;

    async fn bound_user(
        &self,
        external_identity: &NormalizedExternalIdentity,
    ) -> Result<Option<InternalUser>, AppError>;

    async fn bind_new_active_user(
        &self,
        external_identity: NormalizedExternalIdentity,
        now: chrono::DateTime<Utc>,
    ) -> Result<InternalUser, AppError>;

    async fn bind_existing_user(
        &self,
        internal_user_id: Uuid,
        external_identity: NormalizedExternalIdentity,
        now: chrono::DateTime<Utc>,
    ) -> Result<InternalUser, AppError>;

    async fn user_by_id(&self, internal_user_id: Uuid) -> Result<InternalUser, AppError>;

    async fn delete_user(&self, internal_user_id: Uuid) -> Result<(), AppError>;
}

#[derive(Clone)]
pub struct IdentityBindingService {
    repository: Arc<dyn IdentityRepository>,
}

impl IdentityBindingService {
    pub fn new(repository: Arc<dyn IdentityRepository>) -> Self {
        Self { repository }
    }

    pub async fn create_active_user(&self) -> Result<InternalUser, AppError> {
        let user = InternalUser::new_active(Utc::now());
        self.repository.insert_active_user(user.clone()).await?;
        Ok(user)
    }

    pub async fn resolve_identity(
        &self,
        external_identity: NormalizedExternalIdentity,
        binding_mode: BindingMode,
    ) -> Result<InternalUser, AppError> {
        let now = Utc::now();

        if let Some(user) = self.repository.bound_user(&external_identity).await? {
            if user.account_status != AccountStatus::Active {
                return Err(AppError::AccountDisabled);
            }
            return Ok(user);
        }

        match binding_mode {
            BindingMode::LoginOnly => Err(AppError::InvalidCredentials),
            BindingMode::RegisterOrLogin => {
                self.repository
                    .bind_new_active_user(external_identity, now)
                    .await
            }
            BindingMode::LinkToExisting(internal_user_id) => {
                self.repository
                    .bind_existing_user(internal_user_id, external_identity, now)
                    .await
            }
        }
    }

    pub async fn user_by_id(&self, internal_user_id: Uuid) -> Result<InternalUser, AppError> {
        self.repository.user_by_id(internal_user_id).await
    }

    pub async fn delete_user(&self, internal_user_id: Uuid) -> Result<(), AppError> {
        self.repository.delete_user(internal_user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::memory::{InMemoryIdentityRepository, InMemoryState};

    fn test_identity(subject: &str) -> NormalizedExternalIdentity {
        NormalizedExternalIdentity {
            provider_name: "test_provider".to_owned(),
            provider_subject: subject.to_owned(),
            verified_email: None,
            verified_phone: None,
            provider_metadata: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn register_or_login_creates_and_reuses_internal_user() {
        let service = IdentityBindingService::new(Arc::new(InMemoryIdentityRepository::new(
            InMemoryState::shared(),
        )));
        let identity = test_identity("external-user-1");

        let created = service
            .resolve_identity(identity.clone(), BindingMode::RegisterOrLogin)
            .await
            .unwrap();
        let resolved = service
            .resolve_identity(identity, BindingMode::LoginOnly)
            .await
            .unwrap();

        assert_eq!(created.internal_user_id, resolved.internal_user_id);
    }

    #[tokio::test]
    async fn login_only_rejects_unbound_identity() {
        let service = IdentityBindingService::new(Arc::new(InMemoryIdentityRepository::new(
            InMemoryState::shared(),
        )));

        assert!(matches!(
            service
                .resolve_identity(test_identity("missing"), BindingMode::LoginOnly)
                .await,
            Err(AppError::InvalidCredentials)
        ));
    }
}
