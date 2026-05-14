use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    application::error::AppError, config::SessionConfig, domain::session::RefreshTokenRecord,
    providers::local_password::PreparedPasswordChange, security::RefreshTokenHasher,
};

#[async_trait]
pub trait PasswordChangeRepository: Send + Sync {
    async fn change_password_and_rotate_refresh_tokens(
        &self,
        command: PasswordChangeCommand,
    ) -> Result<RefreshTokenRecord, AppError>;
}

pub struct PasswordChangeCommand {
    pub internal_user_id: Uuid,
    pub current_session_id: Uuid,
    pub prepared_password_change: PreparedPasswordChange,
    pub new_token_hash: String,
    pub refresh_token_lifetime_seconds: i64,
    pub now: chrono::DateTime<Utc>,
}

#[derive(Clone)]
pub struct PasswordChangeService {
    repository: Arc<dyn PasswordChangeRepository>,
    session_config: SessionConfig,
    refresh_token_hasher: RefreshTokenHasher,
}

impl PasswordChangeService {
    pub fn new(
        repository: Arc<dyn PasswordChangeRepository>,
        session_config: SessionConfig,
        refresh_token_hasher: RefreshTokenHasher,
    ) -> Self {
        Self {
            repository,
            session_config,
            refresh_token_hasher,
        }
    }

    pub async fn change_password_and_rotate_refresh_tokens(
        &self,
        internal_user_id: Uuid,
        current_session_id: Uuid,
        prepared_password_change: PreparedPasswordChange,
        new_refresh_token_secret: &str,
    ) -> Result<RefreshTokenRecord, AppError> {
        let new_token_hash = self.refresh_token_hasher.hash(new_refresh_token_secret)?;
        self.repository
            .change_password_and_rotate_refresh_tokens(PasswordChangeCommand {
                internal_user_id,
                current_session_id,
                prepared_password_change,
                new_token_hash,
                refresh_token_lifetime_seconds: self.session_config.refresh_token_lifetime_seconds,
                now: Utc::now(),
            })
            .await
    }
}
