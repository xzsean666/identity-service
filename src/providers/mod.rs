pub mod local_password;
pub mod supabase;

use async_trait::async_trait;

use crate::{application::error::AppError, domain::identity::NormalizedExternalIdentity};

#[derive(Clone, Debug)]
pub enum ProviderEntryKind {
    DirectCredential,
    TokenExchange,
}

#[derive(Clone, Debug)]
pub struct ProviderDescriptor {
    pub provider_name: &'static str,
    pub feature_key: &'static str,
    pub entry_kind: ProviderEntryKind,
    pub enabled: bool,
}

#[async_trait]
pub trait IdentityProviderAdapter: Send + Sync {
    fn descriptor(&self) -> ProviderDescriptor;

    async fn verify(
        &self,
        request: ProviderVerificationRequest,
    ) -> Result<NormalizedExternalIdentity, AppError>;
}

#[derive(Clone, Debug)]
pub enum ProviderVerificationRequest {
    LocalPassword { username: String, password: String },
    SupabaseToken { access_token: String },
}
