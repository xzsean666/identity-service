use crate::api::auth::ExchangeRequest;
use crate::api::middleware::AuthenticatedUser;
use crate::error::AppError;
use crate::models::{Identity, UpdateUserDto, UserResponse};
use crate::services::{IdentityService, UserService};
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct CurrentUserResponse {
    pub user: UserResponse,
    pub identities: Vec<Identity>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LinkResponse {
    pub status: String,
    pub identity: Identity,
}

pub async fn get_me(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
) -> Result<Json<CurrentUserResponse>, AppError> {
    let user_service = UserService::new(state.pool.clone());
    let user = user_service.get_user_by_id(auth.user_id).await?;

    let identity_service = IdentityService::new(state.pool.clone());
    let identities = identity_service.get_user_identities(auth.user_id).await?;

    Ok(Json(CurrentUserResponse {
        user: user.into(),
        identities,
    }))
}

pub async fn update_me(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(payload): Json<UpdateUserDto>,
) -> Result<Json<UserResponse>, AppError> {
    let user_service = UserService::new(state.pool.clone());
    let updated = user_service.update_user(auth.user_id, &payload).await?;

    Ok(Json(updated.into()))
}

pub async fn get_user_by_id(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>, AppError> {
    let user_service = UserService::new(state.pool.clone());
    let user = user_service.get_user_by_id(id).await?;

    Ok(Json(user.into()))
}

pub async fn link_identity(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(payload): Json<ExchangeRequest>,
) -> Result<Json<LinkResponse>, AppError> {
    if payload.provider.trim().is_empty() {
        return Err(AppError::BadRequest("Field 'provider' cannot be empty".to_string()));
    }
    if payload.token.trim().is_empty() {
        return Err(AppError::BadRequest("Field 'token' cannot be empty".to_string()));
    }

    let ext = state.providers.verify(&payload.provider, &payload.token).await?;
    let identity_service = IdentityService::new(state.pool.clone());
    let linked = identity_service.link_identity(auth.user_id, &ext).await?;

    Ok(Json(LinkResponse {
        status: "ok".to_string(),
        identity: linked,
    }))
}
