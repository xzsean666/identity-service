use crate::config::Config;
use crate::db::{
    find_refresh_token, find_user_by_id, revoke_all_user_refresh_tokens, revoke_refresh_token,
    save_refresh_token,
};
use crate::error::AppError;
use crate::services::token::JwtEngine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{Duration, Utc};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

#[derive(Clone)]
pub struct SessionService {
    pool: PgPool,
    jwt_engine: Arc<JwtEngine>,
    refresh_token_ttl: Duration,
    access_token_ttl: u64,
}

impl SessionService {
    pub fn new(pool: PgPool, jwt_engine: Arc<JwtEngine>, config: &Config) -> Self {
        Self {
            pool,
            jwt_engine,
            refresh_token_ttl: Duration::seconds(config.jwt_refresh_token_ttl_seconds as i64),
            access_token_ttl: config.jwt_access_token_ttl_seconds,
        }
    }

    pub fn hash_token(raw_token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(raw_token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn generate_raw_token() -> String {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    pub async fn issue_tokens(
        &self,
        user_id: Uuid,
        email: Option<String>,
    ) -> Result<TokenPair, AppError> {
        let access_token = self.jwt_engine.sign(user_id, email)?;
        let raw_refresh_token = Self::generate_raw_token();
        let token_hash = Self::hash_token(&raw_refresh_token);
        let expires_at = Utc::now() + self.refresh_token_ttl;

        save_refresh_token(&self.pool, user_id, &token_hash, expires_at).await?;

        Ok(TokenPair {
            access_token,
            refresh_token: raw_refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.access_token_ttl,
        })
    }

    pub async fn refresh_tokens(&self, raw_refresh_token: &str) -> Result<TokenPair, AppError> {
        let token_hash = Self::hash_token(raw_refresh_token);

        let stored_token = find_refresh_token(&self.pool, &token_hash)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Invalid refresh token".to_string()))?;

        if stored_token.revoked {
            return Err(AppError::Unauthorized("Refresh token has been revoked".to_string()));
        }

        if stored_token.expires_at < Utc::now() {
            return Err(AppError::Unauthorized("Refresh token has expired".to_string()));
        }

        let user = find_user_by_id(&self.pool, stored_token.user_id)
            .await?
            .ok_or_else(|| AppError::Unauthorized("User associated with token not found".to_string()))?;

        if user.status != "active" {
            return Err(AppError::Forbidden(format!(
                "User account is {} (not active)",
                user.status
            )));
        }

        // Revoke the old refresh token (rotation)
        revoke_refresh_token(&self.pool, &token_hash).await?;

        // Issue new token pair
        self.issue_tokens(user.id, user.email).await
    }

    pub async fn revoke_token(&self, raw_refresh_token: &str) -> Result<bool, AppError> {
        let token_hash = Self::hash_token(raw_refresh_token);
        revoke_refresh_token(&self.pool, &token_hash).await
    }

    pub async fn revoke_all_for_user(&self, user_id: Uuid) -> Result<u64, AppError> {
        revoke_all_user_refresh_tokens(&self.pool, user_id).await
    }
}
