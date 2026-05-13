use async_trait::async_trait;
use chrono::Utc;
use jsonwebtoken::{
    Algorithm, DecodingKey, Validation, decode, decode_header,
    jwk::{AlgorithmParameters, Jwk, JwkSet, KeyAlgorithm},
};
use serde::{Deserialize, Serialize};

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
    http_client: reqwest::Client,
}

impl SupabaseProvider {
    pub fn new(config: SupabaseProviderConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    async fn verify_jwt(&self, access_token: &str) -> Result<NormalizedExternalIdentity, AppError> {
        let header =
            decode_header(access_token).map_err(|_| AppError::ProviderVerificationFailed)?;
        let key_id = header
            .kid
            .as_deref()
            .ok_or(AppError::ProviderVerificationFailed)?;
        let jwks = self.load_jwks().await?;
        let jwk = jwks
            .find(key_id)
            .ok_or(AppError::ProviderVerificationFailed)?;
        self.validate_jwk_for_header(jwk, header.alg)?;
        let decoding_key =
            DecodingKey::from_jwk(jwk).map_err(|_| AppError::ProviderVerificationFailed)?;

        let mut validation = Validation::new(header.alg);
        validation.set_audience(&[self.config.audience.clone()]);
        validation.set_issuer(&[self.config.issuer.clone()]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);

        let claims = decode::<SupabaseJwtClaims>(access_token, &decoding_key, &validation)
            .map_err(|_| AppError::ProviderVerificationFailed)?
            .claims;
        if claims.sub.trim().is_empty() {
            return Err(AppError::ProviderVerificationFailed);
        }

        Ok(NormalizedExternalIdentity {
            provider_name: SUPABASE_PROVIDER.to_owned(),
            provider_subject: claims.sub,
            verified_email: claims.email,
            verified_phone: claims.phone,
            provider_metadata: serde_json::json!({}),
        })
    }

    async fn load_jwks(&self) -> Result<JwkSet, AppError> {
        if let Some(jwks_json) = &self.config.jwks_json {
            return serde_json::from_str(jwks_json)
                .map_err(|_| AppError::ProviderVerificationFailed);
        }

        self.http_client
            .get(&self.config.jwks_url)
            .send()
            .await
            .map_err(|_| AppError::ProviderVerificationFailed)?
            .error_for_status()
            .map_err(|_| AppError::ProviderVerificationFailed)?
            .json::<JwkSet>()
            .await
            .map_err(|_| AppError::ProviderVerificationFailed)
    }

    fn fixture_tokens_enabled(&self) -> bool {
        self.config.fixture_tokens_enabled
    }

    fn validate_jwk_for_header(
        &self,
        jwk: &Jwk,
        header_algorithm: Algorithm,
    ) -> Result<(), AppError> {
        let Some(key_algorithm) = jwk.common.key_algorithm else {
            return Err(AppError::ProviderVerificationFailed);
        };
        if !key_algorithm_matches_header(key_algorithm, header_algorithm) {
            return Err(AppError::ProviderVerificationFailed);
        }
        if matches!(jwk.algorithm, AlgorithmParameters::OctetKey(_))
            && self.config.jwks_json.is_none()
        {
            return Err(AppError::ProviderVerificationFailed);
        }
        Ok(())
    }
}

fn key_algorithm_matches_header(key_algorithm: KeyAlgorithm, header_algorithm: Algorithm) -> bool {
    matches!(
        (key_algorithm, header_algorithm),
        (KeyAlgorithm::HS256, Algorithm::HS256)
            | (KeyAlgorithm::HS384, Algorithm::HS384)
            | (KeyAlgorithm::HS512, Algorithm::HS512)
            | (KeyAlgorithm::RS256, Algorithm::RS256)
            | (KeyAlgorithm::RS384, Algorithm::RS384)
            | (KeyAlgorithm::RS512, Algorithm::RS512)
            | (KeyAlgorithm::PS256, Algorithm::PS256)
            | (KeyAlgorithm::PS384, Algorithm::PS384)
            | (KeyAlgorithm::PS512, Algorithm::PS512)
            | (KeyAlgorithm::ES256, Algorithm::ES256)
            | (KeyAlgorithm::ES384, Algorithm::ES384)
            | (KeyAlgorithm::EdDSA, Algorithm::EdDSA)
    )
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
            jwks_url: "https://example.supabase.co/auth/v1/.well-known/jwks.json".to_owned(),
            jwks_json: None,
            fixture_tokens_enabled: true,
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

    #[tokio::test]
    async fn jwt_token_verifies_with_configured_jwks() {
        use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

        let secret = b"supabase-test-secret";
        let jwks_json = serde_json::json!({
            "keys": [{
                "kty": "oct",
                "k": "c3VwYWJhc2UtdGVzdC1zZWNyZXQ",
                "kid": "supabase-test-key",
                "alg": "HS256",
                "use": "sig"
            }]
        })
        .to_string();
        let provider = SupabaseProvider::new(SupabaseProviderConfig {
            enabled: true,
            auto_provision_enabled: true,
            project_url: "https://example.supabase.co".to_owned(),
            issuer: "https://example.supabase.co/auth/v1".to_owned(),
            audience: "authenticated".to_owned(),
            jwks_url: "https://example.supabase.co/auth/v1/.well-known/jwks.json".to_owned(),
            jwks_json: Some(jwks_json),
            fixture_tokens_enabled: false,
        });
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some("supabase-test-key".to_owned());
        let claims = SupabaseJwtClaims {
            sub: "supabase-user-2".to_owned(),
            exp: Utc::now().timestamp() + 300,
            iss: "https://example.supabase.co/auth/v1".to_owned(),
            aud: "authenticated".to_owned(),
            email: Some("jwt-user@example.com".to_owned()),
            phone: Some("+15555550100".to_owned()),
        };
        let access_token = encode(&header, &claims, &EncodingKey::from_secret(secret)).unwrap();

        let identity = provider
            .verify(ProviderVerificationRequest::SupabaseToken { access_token })
            .await
            .unwrap();

        assert_eq!(identity.provider_name, SUPABASE_PROVIDER);
        assert_eq!(identity.provider_subject, "supabase-user-2");
        assert_eq!(
            identity.verified_email.as_deref(),
            Some("jwt-user@example.com")
        );
        assert_eq!(identity.verified_phone.as_deref(), Some("+15555550100"));
    }

    #[tokio::test]
    async fn jwt_token_rejects_wrong_audience() {
        use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};

        let jwks_json = serde_json::json!({
            "keys": [{
                "kty": "oct",
                "k": "c3VwYWJhc2UtdGVzdC1zZWNyZXQ",
                "kid": "supabase-test-key",
                "alg": "HS256",
                "use": "sig"
            }]
        })
        .to_string();
        let provider = SupabaseProvider::new(SupabaseProviderConfig {
            enabled: true,
            auto_provision_enabled: true,
            project_url: "https://example.supabase.co".to_owned(),
            issuer: "https://example.supabase.co/auth/v1".to_owned(),
            audience: "authenticated".to_owned(),
            jwks_url: "https://example.supabase.co/auth/v1/.well-known/jwks.json".to_owned(),
            jwks_json: Some(jwks_json),
            fixture_tokens_enabled: false,
        });
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some("supabase-test-key".to_owned());
        let claims = SupabaseJwtClaims {
            sub: "supabase-user-2".to_owned(),
            exp: Utc::now().timestamp() + 300,
            iss: "https://example.supabase.co/auth/v1".to_owned(),
            aud: "wrong-audience".to_owned(),
            email: None,
            phone: None,
        };
        let access_token = encode(
            &header,
            &claims,
            &EncodingKey::from_secret(b"supabase-test-secret"),
        )
        .unwrap();

        let result = provider
            .verify(ProviderVerificationRequest::SupabaseToken { access_token })
            .await;

        assert!(matches!(result, Err(AppError::ProviderVerificationFailed)));
    }

    #[test]
    fn jwk_algorithm_must_match_token_header_algorithm() {
        let provider = SupabaseProvider::new(SupabaseProviderConfig {
            enabled: true,
            auto_provision_enabled: true,
            project_url: "https://example.supabase.co".to_owned(),
            issuer: "https://example.supabase.co/auth/v1".to_owned(),
            audience: "authenticated".to_owned(),
            jwks_url: "https://example.supabase.co/auth/v1/.well-known/jwks.json".to_owned(),
            jwks_json: Some("{}".to_owned()),
            fixture_tokens_enabled: false,
        });
        let jwk: Jwk = serde_json::from_value(serde_json::json!({
            "kty": "oct",
            "k": "c3VwYWJhc2UtdGVzdC1zZWNyZXQ",
            "kid": "supabase-test-key",
            "alg": "HS256",
            "use": "sig"
        }))
        .unwrap();

        let result = provider.validate_jwk_for_header(&jwk, Algorithm::RS256);

        assert!(matches!(result, Err(AppError::ProviderVerificationFailed)));
    }

    #[test]
    fn remote_jwks_rejects_octet_shared_secret_keys() {
        let provider = SupabaseProvider::new(SupabaseProviderConfig {
            enabled: true,
            auto_provision_enabled: true,
            project_url: "https://example.supabase.co".to_owned(),
            issuer: "https://example.supabase.co/auth/v1".to_owned(),
            audience: "authenticated".to_owned(),
            jwks_url: "https://example.supabase.co/auth/v1/.well-known/jwks.json".to_owned(),
            jwks_json: None,
            fixture_tokens_enabled: false,
        });
        let jwk: Jwk = serde_json::from_value(serde_json::json!({
            "kty": "oct",
            "k": "c3VwYWJhc2UtdGVzdC1zZWNyZXQ",
            "kid": "supabase-test-key",
            "alg": "HS256",
            "use": "sig"
        }))
        .unwrap();

        let result = provider.validate_jwk_for_header(&jwk, Algorithm::HS256);

        assert!(matches!(result, Err(AppError::ProviderVerificationFailed)));
    }

    #[tokio::test]
    async fn fixture_token_is_rejected_for_non_example_project() {
        let provider = SupabaseProvider::new(SupabaseProviderConfig {
            enabled: true,
            auto_provision_enabled: true,
            project_url: "https://real-project.supabase.co".to_owned(),
            issuer: "https://real-project.supabase.co/auth/v1".to_owned(),
            audience: "authenticated".to_owned(),
            jwks_url: "https://real-project.supabase.co/auth/v1/.well-known/jwks.json".to_owned(),
            jwks_json: None,
            fixture_tokens_enabled: false,
        });
        let access_token = serde_json::json!({
            "sub": "supabase-user-1",
            "exp": Utc::now().timestamp() + 300,
            "iss": "https://real-project.supabase.co/auth/v1",
            "aud": "authenticated"
        })
        .to_string();

        let result = provider
            .verify(ProviderVerificationRequest::SupabaseToken { access_token })
            .await;

        assert!(matches!(result, Err(AppError::ProviderVerificationFailed)));
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct SupabaseJwtClaims {
    sub: String,
    exp: i64,
    iss: String,
    aud: String,
    email: Option<String>,
    phone: Option<String>,
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

        if let Ok(identity) = self.verify_jwt(&access_token).await {
            return Ok(identity);
        }
        if !self.fixture_tokens_enabled() {
            return Err(AppError::ProviderVerificationFailed);
        }

        // Local fixture fallback: accept a JSON token payload for tests/dev only.
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
