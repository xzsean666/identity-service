use chrono::Utc;
use uuid::Uuid;

use crate::{
    application::error::AppError,
    domain::{
        identity::{BindingMode, ExternalIdentity, NormalizedExternalIdentity},
        user::{AccountStatus, InternalUser},
    },
    infrastructure::memory::SharedState,
};

#[derive(Clone)]
pub struct IdentityBindingService {
    state: SharedState,
}

impl IdentityBindingService {
    pub fn new(state: SharedState) -> Self {
        Self { state }
    }

    pub fn create_active_user(&self) -> InternalUser {
        let user = InternalUser::new_active(Utc::now());
        let mut state = self.state.lock();
        state.users.insert(user.internal_user_id, user.clone());
        user
    }

    pub fn resolve_identity(
        &self,
        external_identity: NormalizedExternalIdentity,
        binding_mode: BindingMode,
    ) -> Result<InternalUser, AppError> {
        let key = (
            external_identity.provider_name.clone(),
            external_identity.provider_subject.clone(),
        );
        let now = Utc::now();
        let mut state = self.state.lock();

        if let Some(binding) = state.identities_by_provider_subject.get(&key) {
            let user = state
                .users
                .get(&binding.internal_user_id)
                .cloned()
                .ok_or_else(|| {
                    AppError::Internal("identity binding points to missing user".to_owned())
                })?;
            if user.account_status != AccountStatus::Active {
                return Err(AppError::AccountDisabled);
            }
            return Ok(user);
        }

        match binding_mode {
            BindingMode::LoginOnly => Err(AppError::InvalidCredentials),
            BindingMode::RegisterOrLogin => {
                let user = InternalUser::new_active(now);
                let binding = ExternalIdentity {
                    provider_name: external_identity.provider_name,
                    provider_subject: external_identity.provider_subject,
                    internal_user_id: user.internal_user_id,
                    provider_metadata: external_identity.provider_metadata,
                    created_at: now,
                    updated_at: now,
                };
                state.users.insert(user.internal_user_id, user.clone());
                state.identities_by_provider_subject.insert(key, binding);
                Ok(user)
            }
            BindingMode::LinkToExisting(internal_user_id) => {
                let user = state
                    .users
                    .get(&internal_user_id)
                    .cloned()
                    .ok_or(AppError::Unauthorized)?;
                let binding = ExternalIdentity {
                    provider_name: external_identity.provider_name,
                    provider_subject: external_identity.provider_subject,
                    internal_user_id,
                    provider_metadata: external_identity.provider_metadata,
                    created_at: now,
                    updated_at: now,
                };
                state.identities_by_provider_subject.insert(key, binding);
                Ok(user)
            }
        }
    }

    pub fn user_by_id(&self, internal_user_id: Uuid) -> Result<InternalUser, AppError> {
        let state = self.state.lock();
        state
            .users
            .get(&internal_user_id)
            .cloned()
            .ok_or(AppError::Unauthorized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::memory::InMemoryState;

    fn test_identity(subject: &str) -> NormalizedExternalIdentity {
        NormalizedExternalIdentity {
            provider_name: "test_provider".to_owned(),
            provider_subject: subject.to_owned(),
            verified_email: None,
            verified_phone: None,
            provider_metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn register_or_login_creates_and_reuses_internal_user() {
        let service = IdentityBindingService::new(InMemoryState::shared());
        let identity = test_identity("external-user-1");

        let created = service
            .resolve_identity(identity.clone(), BindingMode::RegisterOrLogin)
            .unwrap();
        let resolved = service
            .resolve_identity(identity, BindingMode::LoginOnly)
            .unwrap();

        assert_eq!(created.internal_user_id, resolved.internal_user_id);
    }

    #[test]
    fn login_only_rejects_unbound_identity() {
        let service = IdentityBindingService::new(InMemoryState::shared());

        assert!(matches!(
            service.resolve_identity(test_identity("missing"), BindingMode::LoginOnly),
            Err(AppError::InvalidCredentials)
        ));
    }
}
