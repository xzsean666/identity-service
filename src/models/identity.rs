use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromRow)]
pub struct Identity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalIdentity {
    pub provider: String,
    pub provider_user_id: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthProvider {
    Google,
    X,
    Supabase,
    Mock,
}

impl AuthProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthProvider::Google => "google",
            AuthProvider::X => "x",
            AuthProvider::Supabase => "supabase",
            AuthProvider::Mock => "mock",
        }
    }
}

impl std::str::FromStr for AuthProvider {
    type Err = crate::error::AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "google" => Ok(AuthProvider::Google),
            "x" | "twitter" => Ok(AuthProvider::X),
            "supabase" => Ok(AuthProvider::Supabase),
            "mock" => Ok(AuthProvider::Mock),
            other => Err(crate::error::AppError::BadRequest(format!(
                "Unsupported authentication provider: {}",
                other
            ))),
        }
    }
}
