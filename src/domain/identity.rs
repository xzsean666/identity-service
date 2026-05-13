use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub const LOCAL_PASSWORD_PROVIDER: &str = "local_password";
pub const SUPABASE_PROVIDER: &str = "supabase";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalIdentity {
    pub provider_name: String,
    pub provider_subject: String,
    pub internal_user_id: Uuid,
    pub provider_metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NormalizedExternalIdentity {
    pub provider_name: String,
    pub provider_subject: String,
    pub verified_email: Option<String>,
    pub verified_phone: Option<String>,
    pub provider_metadata: Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingMode {
    LoginOnly,
    RegisterOrLogin,
    LinkToExisting(Uuid),
}

impl NormalizedExternalIdentity {
    pub fn local_password(local_credential_id: Uuid, username: &str) -> Self {
        Self {
            provider_name: LOCAL_PASSWORD_PROVIDER.to_owned(),
            provider_subject: local_credential_id.to_string(),
            verified_email: None,
            verified_phone: None,
            provider_metadata: serde_json::json!({ "username": username }),
        }
    }
}
