use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionStatus {
    Active,
    Revoked,
    Expired,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RefreshTokenStatus {
    Active,
    Consumed,
    Revoked,
    Reused,
    Expired,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub session_id: Uuid,
    pub internal_user_id: Uuid,
    pub provider_name: String,
    pub client_id: String,
    pub device_metadata: Option<serde_json::Value>,
    pub status: SessionStatus,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefreshTokenRecord {
    pub refresh_token_id: Uuid,
    pub session_id: Uuid,
    pub internal_user_id: Uuid,
    pub token_family_id: Uuid,
    pub token_hash: String,
    pub status: RefreshTokenStatus,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}
