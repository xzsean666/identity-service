use axum::{extract::State, Json};
use crate::services::token::{JwksResponse, JwtEngine};

pub async fn get_jwks(State(jwt_engine): State<JwtEngine>) -> Json<JwksResponse> {
    Json(jwt_engine.jwks())
}
