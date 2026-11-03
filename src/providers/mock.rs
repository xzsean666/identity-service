use crate::error::AppError;
use crate::models::ExternalIdentity;
use crate::providers::{BoxFuture, ProviderVerifier};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockPayload {
    pub sub: Option<String>,
    pub id: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

pub struct MockProvider {
    predefined: RwLock<HashMap<String, ExternalIdentity>>,
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            predefined: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, token: &str, identity: ExternalIdentity) {
        let mut map = self.predefined.write().unwrap();
        map.insert(token.to_string(), identity);
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderVerifier for MockProvider {
    fn provider_name(&self) -> &'static str {
        "mock"
    }

    fn verify<'a>(&'a self, token: &'a str) -> BoxFuture<'a, Result<ExternalIdentity, AppError>> {
        Box::pin(async move {
            if token == "invalid" || token == "invalid_token" || token == "reject" {
                return Err(AppError::Unauthorized("Invalid mock token".to_string()));
            }

            // Check registered tokens first
            {
                let map = self.predefined.read().unwrap();
                if let Some(ident) = map.get(token) {
                    return Ok(ident.clone());
                }
            }

            // If token is JSON
            if token.starts_with('{') {
                if let Ok(payload) = serde_json::from_str::<MockPayload>(token) {
                    let user_id = payload
                        .sub
                        .or(payload.id)
                        .unwrap_or_else(|| "mock_user".to_string());
                    return Ok(ExternalIdentity {
                        provider: "mock".to_string(),
                        provider_user_id: user_id,
                        email: payload.email,
                        name: payload.name,
                        avatar_url: payload.avatar_url,
                    });
                }
            }

            // Format: "mock:<user_id>:<email>:<name>"
            if let Some(rest) = token.strip_prefix("mock:") {
                let parts: Vec<&str> = rest.split(':').collect();
                let user_id = parts.first().unwrap_or(&"user_123").to_string();
                let email = parts.get(1).map(|s| s.to_string());
                let name = parts.get(2).map(|s| s.to_string());
                return Ok(ExternalIdentity {
                    provider: "mock".to_string(),
                    provider_user_id: user_id,
                    email,
                    name,
                    avatar_url: None,
                });
            }

            // Default fallback for any non-empty string
            if token.trim().is_empty() {
                return Err(AppError::BadRequest("Token cannot be empty".to_string()));
            }

            Ok(ExternalIdentity {
                provider: "mock".to_string(),
                provider_user_id: token.to_string(),
                email: Some(format!("{}@example.com", token)),
                name: Some(format!("Mock User {}", token)),
                avatar_url: None,
            })
        })
    }
}
