use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    application::{
        auth::{AuthResponse, AuthService},
        error::AppError,
    },
    domain::user::InternalUser,
};

#[derive(Clone)]
pub struct HttpState {
    pub auth_service: Arc<AuthService>,
}

pub fn router(auth_service: Arc<AuthService>) -> Router {
    let state = HttpState { auth_service };
    Router::new()
        .route("/health", get(health))
        .route("/v1/auth/register", post(register))
        .route("/v1/auth/login", post(login))
        .route("/v1/auth/password/change", post(change_password))
        .route("/v1/auth/supabase/exchange", post(exchange_supabase))
        .route("/v1/auth/refresh", post(refresh))
        .route("/v1/auth/logout", post(logout))
        .route("/v1/users/me", get(current_user))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

#[derive(Deserialize)]
struct PasswordAuthRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct PasswordChangeRequest {
    current_password: String,
    new_password: String,
}

#[derive(Deserialize)]
struct SupabaseExchangeRequest {
    access_token: String,
}

#[derive(Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Serialize)]
struct TokenResponse {
    tokens: crate::domain::token::TokenPair,
}

#[derive(Serialize)]
struct LogoutResponse {
    revoked: bool,
}

async fn register(
    State(state): State<HttpState>,
    Json(request): Json<PasswordAuthRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    state
        .auth_service
        .register_with_local_password(request.username, request.password)
        .await
        .map(Json)
}

async fn login(
    State(state): State<HttpState>,
    Json(request): Json<PasswordAuthRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    state
        .auth_service
        .login_with_local_password(request.username, request.password)
        .await
        .map(Json)
}

async fn change_password(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(request): Json<PasswordChangeRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    let access_token = bearer_token(&headers)?;
    let tokens = state
        .auth_service
        .change_local_password(
            &access_token,
            request.current_password,
            request.new_password,
        )
        .await?;
    Ok(Json(TokenResponse { tokens }))
}

async fn exchange_supabase(
    State(state): State<HttpState>,
    Json(request): Json<SupabaseExchangeRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    state
        .auth_service
        .exchange_supabase_token(request.access_token)
        .await
        .map(Json)
}

async fn refresh(
    State(state): State<HttpState>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    let tokens = state.auth_service.refresh(request.refresh_token).await?;
    Ok(Json(TokenResponse { tokens }))
}

async fn logout(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<LogoutResponse>, AppError> {
    let access_token = bearer_token(&headers)?;
    state.auth_service.logout(&access_token).await?;
    Ok(Json(LogoutResponse { revoked: true }))
}

async fn current_user(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<InternalUser>, AppError> {
    let access_token = bearer_token(&headers)?;
    state
        .auth_service
        .current_user(&access_token)
        .await
        .map(Json)
}

fn bearer_token(headers: &HeaderMap) -> Result<String, AppError> {
    let header_value = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    let Some(token) = header_value.strip_prefix("Bearer ") else {
        return Err(AppError::Unauthorized);
    };
    Ok(token.to_owned())
}
