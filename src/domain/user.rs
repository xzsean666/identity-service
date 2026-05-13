use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccountStatus {
    Active,
    Disabled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InternalUser {
    pub internal_user_id: Uuid,
    pub account_status: AccountStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl InternalUser {
    pub fn new_active(now: DateTime<Utc>) -> Self {
        Self {
            internal_user_id: Uuid::new_v4(),
            account_status: AccountStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }
}
