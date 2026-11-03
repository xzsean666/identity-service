use crate::error::AppError;
use crate::models::UserResponse;
use crate::services::{IdentityService, SessionService, TokenPair};
use crate::state::AppState;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ExchangeRequest {
    pub provider: String,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExchangeResponse {
    pub user: UserResponse,
    pub tokens: TokenPair,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct RevokeRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RevokeResponse {
    pub message: &'static str,
}

pub async fn exchange(
    State(state): State<AppState>,
    Json(payload): Json<ExchangeRequest>,
) -> Result<Json<ExchangeResponse>, AppError> {
    if payload.provider.trim().is_empty() {
        return Err(AppError::BadRequest("Field 'provider' cannot be empty".to_string()));
    }
    if payload.token.trim().is_empty() {
        return Err(AppError::BadRequest("Field 'token' cannot be empty".to_string()));
    }

    // 1. Verify token with provider
    let ext_identity = state.providers.verify(&payload.provider, &payload.token).await?;

    // 2. Resolve or create user
    let identity_service = IdentityService::new(state.pool.clone());
    let user = identity_service.resolve_or_create_user(&ext_identity).await?;

    // 3. Issue token pair
    let session_service = SessionService::new(state.pool.clone(), state.jwt_engine.clone(), &state.config);
    let tokens = session_service.issue_tokens(user.id, user.email.clone()).await?;

    Ok(Json(ExchangeResponse {
        user: user.into(),
        tokens,
    }))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<TokenPair>, AppError> {
    if payload.refresh_token.trim().is_empty() {
        return Err(AppError::BadRequest("Field 'refresh_token' cannot be empty".to_string()));
    }

    let session_service = SessionService::new(state.pool.clone(), state.jwt_engine.clone(), &state.config);
    let tokens = session_service.refresh_tokens(&payload.refresh_token).await?;

    Ok(Json(tokens))
}

pub async fn revoke(
    State(state): State<AppState>,
    Json(payload): Json<RevokeRequest>,
) -> Result<Json<RevokeResponse>, AppError> {
    if payload.refresh_token.trim().is_empty() {
        return Err(AppError::BadRequest("Field 'refresh_token' cannot be empty".to_string()));
    }

    let session_service = SessionService::new(state.pool.clone(), state.jwt_engine.clone(), &state.config);
    session_service.revoke_token(&payload.refresh_token).await?;

    Ok(Json(RevokeResponse {
        message: "Successfully revoked token",
    }))
}
