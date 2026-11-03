use crate::api::auth::{exchange, refresh, revoke};
use crate::api::docs::docs_router;
use crate::api::jwks::get_jwks;
use crate::api::users::{get_me, get_user_by_id, link_identity, update_me};
use crate::state::AppState;
use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub timestamp: u64,
}

pub async fn health_check() -> Json<HealthResponse> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        timestamp: now,
    })
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/healthz", get(health_check))
        .route("/health", get(health_check))
        .route("/.well-known/jwks.json", get(get_jwks))
        .route("/v1/auth/exchange", post(exchange))
        .route("/v1/auth/refresh", post(refresh))
        .route("/v1/auth/revoke", post(revoke))
        .route("/v1/users/me", get(get_me).patch(update_me))
        .route("/v1/users/link", post(link_identity))
        .route("/v1/users/{id}", get(get_user_by_id))
        .merge(docs_router())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
