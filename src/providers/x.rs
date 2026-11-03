use crate::error::AppError;
use crate::models::ExternalIdentity;
use crate::providers::{BoxFuture, ProviderVerifier};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct XUserData {
    pub id: String,
    pub name: String,
    pub username: Option<String>,
    pub profile_image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct XUserResponse {
    pub data: Option<XUserData>,
    pub detail: Option<String>,
    pub title: Option<String>,
}

pub struct XProvider {
    client: Client,
    _client_id: Option<String>,
    api_url: String,
}

impl XProvider {
    pub fn new(client_id: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        Self {
            client,
            _client_id: client_id,
            api_url: "https://api.twitter.com/2/users/me".to_string(),
        }
    }

    #[cfg(test)]
    pub fn with_base_url(mut self, url: String) -> Self {
        self.api_url = url;
        self
    }
}

impl ProviderVerifier for XProvider {
    fn provider_name(&self) -> &'static str {
        "x"
    }

    fn verify<'a>(&'a self, token: &'a str) -> BoxFuture<'a, Result<ExternalIdentity, AppError>> {
        Box::pin(async move {
            let res = self
                .client
                .get(&self.api_url)
                .bearer_auth(token)
                .send()
                .await
                .map_err(|e| AppError::Provider(format!("Failed to connect to X (Twitter): {e}")))?;

            if !res.status().is_success() {
                let err_text = res.text().await.unwrap_or_default();
                return Err(AppError::Unauthorized(format!("Invalid X token: {err_text}")));
            }

            let body: XUserResponse = res
                .json()
                .await
                .map_err(|e| AppError::Provider(format!("Invalid X API response: {e}")))?;

            let data = body.data.ok_or_else(|| {
                AppError::Unauthorized(body.detail.unwrap_or_else(|| "Failed to get user data from X".to_string()))
            })?;

            Ok(ExternalIdentity {
                provider: "x".to_string(),
                provider_user_id: data.id,
                email: None,
                name: Some(data.name),
                avatar_url: data.profile_image_url,
            })
        })
    }
}
