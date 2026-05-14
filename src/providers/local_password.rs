use std::sync::Arc;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    application::error::AppError,
    domain::identity::NormalizedExternalIdentity,
    providers::{
        IdentityProviderAdapter, ProviderDescriptor, ProviderEntryKind, ProviderVerificationRequest,
    },
};

#[derive(Clone, Debug)]
pub struct LocalCredential {
    pub credential_id: Uuid,
    pub internal_user_id: Uuid,
    pub username: String,
    pub normalized_username: String,
    pub password_hash: String,
    pub password_hash_algorithm: String,
    pub password_hash_parameters: String,
    pub status: LocalCredentialStatus,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalCredentialStatus {
    Active,
    Disabled,
}

#[async_trait]
pub trait LocalCredentialRepository: Send + Sync {
    async fn create_credential(
        &self,
        normalized_username: &str,
        credential: LocalCredential,
    ) -> Result<LocalCredential, AppError>;

    async fn find_by_normalized_username(
        &self,
        normalized_username: &str,
    ) -> Result<Option<LocalCredential>, AppError>;

    async fn find_by_internal_user_id(
        &self,
        internal_user_id: Uuid,
    ) -> Result<Option<LocalCredential>, AppError>;

    async fn update_for_internal_user_id(
        &self,
        internal_user_id: Uuid,
        credential: LocalCredential,
    ) -> Result<(), AppError>;
}

#[derive(Clone)]
pub struct LocalPasswordProvider {
    credential_repository: Arc<dyn LocalCredentialRepository>,
    enabled: bool,
}

impl LocalPasswordProvider {
    pub fn new(credential_repository: Arc<dyn LocalCredentialRepository>, enabled: bool) -> Self {
        Self {
            credential_repository,
            enabled,
        }
    }

    pub fn normalize_username(username: &str) -> String {
        username.trim().to_lowercase()
    }

    pub fn hash_password(password: &str) -> Result<String, AppError> {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    pub fn verify_password(password: &str, password_hash: &str) -> Result<(), AppError> {
        let parsed_hash =
            PasswordHash::new(password_hash).map_err(|_| AppError::InvalidCredentials)?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::InvalidCredentials)
    }

    pub async fn create_credential_for_user(
        &self,
        internal_user_id: Uuid,
        username: &str,
        password: &str,
    ) -> Result<LocalCredential, AppError> {
        validate_username_and_password(username, password)?;
        let now = Utc::now();
        let normalized_username = Self::normalize_username(username);
        let credential = LocalCredential {
            credential_id: Uuid::new_v4(),
            internal_user_id,
            username: username.trim().to_owned(),
            normalized_username: normalized_username.clone(),
            password_hash: Self::hash_password(password)?,
            password_hash_algorithm: "argon2id".to_owned(),
            password_hash_parameters: "phc_string".to_owned(),
            status: LocalCredentialStatus::Active,
            created_at: now,
            updated_at: now,
        };

        self.credential_repository
            .create_credential(&normalized_username, credential)
            .await
    }

    pub async fn change_password(
        &self,
        internal_user_id: Uuid,
        current_password: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        validate_password(new_password)?;
        let mut credential = self
            .credential_repository
            .find_by_internal_user_id(internal_user_id)
            .await?
            .ok_or(AppError::InvalidCredentials)?;

        if credential.status != LocalCredentialStatus::Active {
            return Err(AppError::InvalidCredentials);
        }
        Self::verify_password(current_password, &credential.password_hash)?;
        credential.password_hash = Self::hash_password(new_password)?;
        credential.updated_at = Utc::now();
        self.credential_repository
            .update_for_internal_user_id(internal_user_id, credential)
            .await
    }
}

#[async_trait]
impl IdentityProviderAdapter for LocalPasswordProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            provider_name: "local_password",
            feature_key: "identity_providers.local_password.enabled",
            entry_kind: ProviderEntryKind::DirectCredential,
            enabled: self.enabled,
        }
    }

    async fn verify(
        &self,
        request: ProviderVerificationRequest,
    ) -> Result<NormalizedExternalIdentity, AppError> {
        let ProviderVerificationRequest::LocalPassword { username, password } = request else {
            return Err(AppError::ProviderVerificationFailed);
        };
        validate_username_and_password(&username, &password)?;

        let normalized_username = Self::normalize_username(&username);
        let credential = self
            .credential_repository
            .find_by_normalized_username(&normalized_username)
            .await?
            .ok_or(AppError::InvalidCredentials)?;

        if credential.status != LocalCredentialStatus::Active {
            return Err(AppError::InvalidCredentials);
        }
        Self::verify_password(&password, &credential.password_hash)?;
        Ok(NormalizedExternalIdentity::local_password(
            credential.credential_id,
            &credential.username,
        ))
    }
}

pub type SharedLocalPasswordProvider = Arc<LocalPasswordProvider>;

fn validate_username_and_password(username: &str, password: &str) -> Result<(), AppError> {
    let normalized_username = LocalPasswordProvider::normalize_username(username);
    if normalized_username.len() < 3 {
        return Err(AppError::ValidationFailed);
    }
    validate_password(password)
}

fn validate_password(password: &str) -> Result<(), AppError> {
    if password.len() < 8 {
        return Err(AppError::ValidationFailed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::memory::{InMemoryLocalCredentialRepository, InMemoryState};

    #[test]
    fn password_hash_verifies_correct_password_only() {
        let password_hash = LocalPasswordProvider::hash_password("correct-password").unwrap();

        assert!(LocalPasswordProvider::verify_password("correct-password", &password_hash).is_ok());
        assert!(matches!(
            LocalPasswordProvider::verify_password("wrong-password", &password_hash),
            Err(AppError::InvalidCredentials)
        ));
    }

    #[tokio::test]
    async fn change_password_requires_current_password_and_updates_hash() {
        let state = InMemoryState::shared();
        let provider = LocalPasswordProvider::new(
            Arc::new(InMemoryLocalCredentialRepository::new(state.clone())),
            true,
        );
        let internal_user_id = Uuid::new_v4();
        provider
            .create_credential_for_user(internal_user_id, "Alice", "old-password")
            .await
            .unwrap();

        assert!(matches!(
            provider
                .change_password(internal_user_id, "wrong-password", "new-password")
                .await,
            Err(AppError::InvalidCredentials)
        ));

        provider
            .change_password(internal_user_id, "old-password", "new-password")
            .await
            .unwrap();

        let state = state.lock();
        let credential = state
            .local_credentials_by_username
            .get("alice")
            .expect("credential must exist");
        assert!(
            LocalPasswordProvider::verify_password("new-password", &credential.password_hash)
                .is_ok()
        );
    }
}
