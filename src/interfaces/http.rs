use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::{
        HeaderMap, HeaderValue, Method, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
    response::IntoResponse,
    routing::{get, post},
};
use jsonwebtoken::jwk::JwkSet;
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use crate::{
    application::{auth::AuthResponse, bootstrap::ApplicationServices, error::AppError},
    config::FrontendDirectConfig,
    domain::user::InternalUser,
};

#[derive(Clone)]
pub struct HttpState {
    pub services: Arc<ApplicationServices>,
}

pub fn router(services: Arc<ApplicationServices>) -> Router {
    let frontend_direct = services.http.frontend_direct.clone();
    let state = HttpState { services };
    let router = Router::new()
        .route("/health", get(health))
        .route("/ready", get(readiness))
        .route("/.well-known/jwks.json", get(jwks))
        .route("/v1/auth/register", post(register))
        .route("/v1/auth/login", post(login))
        .route("/v1/auth/password/change", post(change_password))
        .route("/v1/auth/supabase/exchange", post(exchange_supabase))
        .route("/v1/auth/refresh", post(refresh))
        .route("/v1/auth/logout", post(logout))
        .route("/v1/users/me", get(current_user))
        .with_state(state);

    if frontend_direct.enabled {
        router.layer(frontend_direct_cors_layer(&frontend_direct))
    } else {
        router
    }
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn readiness(State(state): State<HttpState>) -> Result<impl IntoResponse, AppError> {
    let report = state.services.readiness_service.check().await?;
    Ok((StatusCode::OK, Json(report)))
}

async fn jwks(State(state): State<HttpState>) -> Result<Json<JwkSet>, AppError> {
    state.services.auth_service.public_jwks().map(Json)
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
        .services
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
        .services
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
        .services
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
        .services
        .auth_service
        .exchange_supabase_token(request.access_token)
        .await
        .map(Json)
}

async fn refresh(
    State(state): State<HttpState>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    let tokens = state
        .services
        .auth_service
        .refresh(request.refresh_token)
        .await?;
    Ok(Json(TokenResponse { tokens }))
}

async fn logout(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<LogoutResponse>, AppError> {
    let access_token = bearer_token(&headers)?;
    state.services.auth_service.logout(&access_token).await?;
    Ok(Json(LogoutResponse { revoked: true }))
}

async fn current_user(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<InternalUser>, AppError> {
    let access_token = bearer_token(&headers)?;
    state
        .services
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

fn frontend_direct_cors_layer(config: &FrontendDirectConfig) -> CorsLayer {
    let allowed_origins = config
        .allowed_origins
        .iter()
        .map(|origin| {
            HeaderValue::from_str(origin)
                .expect("frontend allowed origins are validated during configuration loading")
        })
        .collect::<Vec<_>>();

    CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
}
