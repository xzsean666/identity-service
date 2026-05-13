use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;

use crate::{
    application::error::AppError,
    config::SupabaseProviderConfig,
    domain::identity::{NormalizedExternalIdentity, SUPABASE_PROVIDER},
    providers::{
        IdentityProviderAdapter, ProviderDescriptor, ProviderEntryKind, ProviderVerificationRequest,
    },
};

#[derive(Clone)]
pub struct SupabaseProvider {
    config: SupabaseProviderConfig,
}

impl SupabaseProvider {
    pub fn new(config: SupabaseProviderConfig) -> Self {
        Self { config }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::identity::SUPABASE_PROVIDER,
        providers::{IdentityProviderAdapter, ProviderVerificationRequest},
    };

    #[tokio::test]
    async fn fixture_token_normalizes_supabase_identity() {
        let provider = SupabaseProvider::new(SupabaseProviderConfig {
            enabled: true,
            auto_provision_enabled: true,
            project_url: "https://example.supabase.co".to_owned(),
            issuer: "https://example.supabase.co/auth/v1".to_owned(),
            audience: "authenticated".to_owned(),
        });
        let access_token = serde_json::json!({
            "sub": "supabase-user-1",
            "exp": Utc::now().timestamp() + 300,
            "iss": "https://example.supabase.co/auth/v1",
            "aud": "authenticated",
            "email": "user@example.com"
        })
        .to_string();

        let identity = provider
            .verify(ProviderVerificationRequest::SupabaseToken { access_token })
            .await
            .unwrap();

        assert_eq!(identity.provider_name, SUPABASE_PROVIDER);
        assert_eq!(identity.provider_subject, "supabase-user-1");
        assert_eq!(identity.verified_email.as_deref(), Some("user@example.com"));
    }
}

#[derive(Debug, Deserialize)]
struct SupabaseTokenFixture {
    sub: String,
    exp: Option<i64>,
    iss: Option<String>,
    aud: Option<String>,
    email: Option<String>,
}

#[async_trait]
impl IdentityProviderAdapter for SupabaseProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            provider_name: SUPABASE_PROVIDER,
            feature_key: "identity_providers.supabase.enabled",
            entry_kind: ProviderEntryKind::TokenExchange,
            enabled: self.config.enabled,
        }
    }

    async fn verify(
        &self,
        request: ProviderVerificationRequest,
    ) -> Result<NormalizedExternalIdentity, AppError> {
        let ProviderVerificationRequest::SupabaseToken { access_token } = request else {
            return Err(AppError::ProviderVerificationFailed);
        };

        // MVP fixture mode: accept a JSON token payload. Production verification should replace
        // this adapter internals without changing the provider contract.
        let fixture: SupabaseTokenFixture = serde_json::from_str(&access_token)
            .map_err(|_| AppError::ProviderVerificationFailed)?;

        if fixture.sub.trim().is_empty() {
            return Err(AppError::ProviderVerificationFailed);
        }
        if let Some(exp) = fixture.exp {
            if exp < Utc::now().timestamp() {
                return Err(AppError::ProviderVerificationFailed);
            }
        }
        if let Some(issuer) = &fixture.iss {
            if issuer != &self.config.issuer {
                return Err(AppError::ProviderVerificationFailed);
            }
        }
        if let Some(audience) = &fixture.aud {
            if audience != &self.config.audience {
                return Err(AppError::ProviderVerificationFailed);
            }
        }

        Ok(NormalizedExternalIdentity {
            provider_name: SUPABASE_PROVIDER.to_owned(),
            provider_subject: fixture.sub,
            verified_email: fixture.email,
            verified_phone: None,
            provider_metadata: serde_json::json!({}),
        })
    }
}
