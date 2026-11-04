use crate::error::AppError;
use crate::models::ExternalIdentity;
use crate::providers::{BoxFuture, ProviderVerifier};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Deserialize;
use sha2::Sha256;
use std::time::Duration;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Deserialize)]
struct SupabaseUserMetadata {
    pub name: Option<String>,
    pub full_name: Option<String>,
    pub avatar_url: Option<String>,
    pub picture: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SupabaseUserResponse {
    pub id: Option<String>,
    pub sub: Option<String>,
    pub email: Option<String>,
    pub user_metadata: Option<SupabaseUserMetadata>,
    pub error_description: Option<String>,
    pub message: Option<String>,
    pub exp: Option<u64>,
}

pub struct SupabaseProvider {
    client: Client,
    supabase_url: Option<String>,
    jwt_secret: Option<String>,
    is_production: bool,
}

impl SupabaseProvider {
    pub fn new(
        supabase_url: Option<String>,
        jwt_secret: Option<String>,
        is_production: bool,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        Self {
            client,
            supabase_url,
            jwt_secret,
            is_production,
        }
    }

    fn verify_jwt_hs256(token: &str, secret: &str) -> Result<ExternalIdentity, AppError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(AppError::Unauthorized(
                "Invalid Supabase JWT format (expected 3 parts)".to_string(),
            ));
        }

        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let signature_bytes = URL_SAFE_NO_PAD
            .decode(parts[2])
            .map_err(|_| AppError::Unauthorized("Invalid base64 in Supabase JWT signature".to_string()))?;

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| AppError::Config(format!("Invalid HMAC secret: {e}")))?;
        mac.update(signing_input.as_bytes());
        mac.verify_slice(&signature_bytes)
            .map_err(|_| AppError::Unauthorized("Supabase JWT signature verification failed".to_string()))?;

        Self::parse_payload(parts[1])
    }

    fn parse_payload(payload_b64: &str) -> Result<ExternalIdentity, AppError> {
        let payload_bytes = URL_SAFE_NO_PAD
            .decode(payload_b64)
            .map_err(|_| AppError::Unauthorized("Invalid Supabase JWT base64".to_string()))?;

        let user: SupabaseUserResponse = serde_json::from_slice(&payload_bytes)
            .map_err(|_| AppError::Unauthorized("Invalid Supabase JWT payload JSON".to_string()))?;

        if let Some(exp) = user.exp {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if exp < now {
                return Err(AppError::Unauthorized("Supabase JWT has expired".to_string()));
            }
        }

        let provider_user_id = user
            .id
            .or(user.sub)
            .ok_or_else(|| AppError::Unauthorized("Supabase token missing 'sub' or 'id'".to_string()))?;

        let (name, avatar_url) = if let Some(meta) = user.user_metadata {
            (meta.name.or(meta.full_name), meta.avatar_url.or(meta.picture))
        } else {
            (None, None)
        };

        Ok(ExternalIdentity {
            provider: "supabase".to_string(),
            provider_user_id,
            email: user.email,
            name,
            avatar_url,
        })
    }

    fn parse_jwt_claims(token: &str) -> Result<ExternalIdentity, AppError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() < 2 {
            return Err(AppError::Unauthorized("Invalid Supabase JWT format".to_string()));
        }
        Self::parse_payload(parts[1])
    }
}

impl ProviderVerifier for SupabaseProvider {
    fn provider_name(&self) -> &'static str {
        "supabase"
    }

    fn verify<'a>(&'a self, token: &'a str) -> BoxFuture<'a, Result<ExternalIdentity, AppError>> {
        Box::pin(async move {
            // 1. If URL is configured, verify via Supabase Auth API
            if let Some(ref base_url) = self.supabase_url {
                let endpoint = format!("{}/auth/v1/user", base_url.trim_end_matches('/'));
                let mut req = self.client.get(&endpoint).bearer_auth(token);
                req = req.header("apikey", token);

                let res = req
                    .send()
                    .await
                    .map_err(|e| AppError::Provider(format!("Failed to connect to Supabase: {e}")))?;

                if !res.status().is_success() {
                    let err_text = res.text().await.unwrap_or_default();
                    return Err(AppError::Unauthorized(format!("Invalid Supabase token: {err_text}")));
                }

                let user: SupabaseUserResponse = res
                    .json()
                    .await
                    .map_err(|e| AppError::Provider(format!("Invalid Supabase API response: {e}")))?;

                let provider_user_id = user
                    .id
                    .or(user.sub)
                    .ok_or_else(|| AppError::Unauthorized("Supabase token missing user ID".to_string()))?;

                let (name, avatar_url) = if let Some(meta) = user.user_metadata {
                    (meta.name.or(meta.full_name), meta.avatar_url.or(meta.picture))
                } else {
                    (None, None)
                };

                return Ok(ExternalIdentity {
                    provider: "supabase".to_string(),
                    provider_user_id,
                    email: user.email,
                    name,
                    avatar_url,
                });
            }

            // 2. If JWT secret is configured, verify HMAC signature locally
            if let Some(ref secret) = self.jwt_secret {
                return Self::verify_jwt_hs256(token, secret);
            }

            // 3. Neither URL nor Secret configured
            if self.is_production {
                return Err(AppError::Config(
                    "Supabase provider requires SUPABASE_URL or SUPABASE_JWT_SECRET in production"
                        .to_string(),
                ));
            }

            tracing::warn!("Supabase verification falling back to unverified parsing in non-production mode");
            Self::parse_jwt_claims(token)
        })
    }
}
