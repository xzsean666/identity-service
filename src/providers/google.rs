use crate::error::AppError;
use crate::models::ExternalIdentity;
use crate::providers::{BoxFuture, ProviderVerifier};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GoogleTokenInfo {
    pub sub: Option<String>,
    pub email: Option<String>,
    #[serde(default)]
    pub email_verified: Option<serde_json::Value>,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub aud: Option<String>,
    pub error_description: Option<String>,
}

pub struct GoogleProvider {
    client: Client,
    client_id: Option<String>,
    tokeninfo_url: String,
}

impl GoogleProvider {
    pub fn new(client_id: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        Self {
            client,
            client_id,
            tokeninfo_url: "https://oauth2.googleapis.com/tokeninfo".to_string(),
        }
    }

    #[cfg(test)]
    pub fn with_base_url(mut self, url: String) -> Self {
        self.tokeninfo_url = url;
        self
    }
}

impl ProviderVerifier for GoogleProvider {
    fn provider_name(&self) -> &'static str {
        "google"
    }

    fn verify<'a>(&'a self, token: &'a str) -> BoxFuture<'a, Result<ExternalIdentity, AppError>> {
        Box::pin(async move {
            let res = self
                .client
                .get(&self.tokeninfo_url)
                .query(&[("id_token", token)])
                .send()
                .await
                .map_err(|e| AppError::Provider(format!("Failed to connect to Google: {e}")))?;

            if !res.status().is_success() {
                let err_text = res.text().await.unwrap_or_default();
                return Err(AppError::Unauthorized(format!(
                    "Invalid Google token: {err_text}"
                )));
            }

            let info: GoogleTokenInfo = res
                .json()
                .await
                .map_err(|e| AppError::Provider(format!("Invalid Google token response: {e}")))?;

            if let Some(err) = info.error_description {
                return Err(AppError::Unauthorized(format!("Google token error: {err}")));
            }

            let provider_user_id = info
                .sub
                .ok_or_else(|| AppError::Unauthorized("Missing 'sub' in Google token".to_string()))?;

            if let Some(ref expected_aud) = self.client_id {
                if let Some(ref aud) = info.aud {
                    if aud != expected_aud {
                        return Err(AppError::Unauthorized(format!(
                            "Google token audience mismatch: expected '{expected_aud}', got '{aud}'"
                        )));
                    }
                }
            }

            Ok(ExternalIdentity {
                provider: "google".to_string(),
                provider_user_id,
                email: info.email,
                name: info.name,
                avatar_url: info.picture,
            })
        })
    }
}
