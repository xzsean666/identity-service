use crate::api::auth::ExchangeRequest;
use crate::api::middleware::{AuthenticatedUser, OptionalAuthUser};
use crate::error::AppError;
use crate::models::{Identity, PublicUserResponse, UpdateUserDto, UserResponse};
use crate::services::{IdentityService, UserService};
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
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
    auth: OptionalAuthUser,
    Path(id): Path<Uuid>,
) -> Result<Response, AppError> {
    let user_service = UserService::new(state.pool.clone());
    let user = user_service.get_user_by_id(id).await?;

    let is_owner = auth.0.as_ref().map(|a| a.user_id) == Some(id);
    if is_owner {
        Ok(Json(UserResponse::from(user)).into_response())
    } else {
        Ok(Json(PublicUserResponse::from(user)).into_response())
    }
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
