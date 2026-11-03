use crate::error::AppError;
use crate::services::token::{Claims, JwtEngine};
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header::AUTHORIZATION, request::Parts},
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub email: Option<String>,
    pub claims: Claims,
}

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
    JwtEngine: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

        let token = if let Some(t) = auth_header.strip_prefix("Bearer ") {
            t.trim()
        } else if let Some(t) = auth_header.strip_prefix("bearer ") {
            t.trim()
        } else {
            return Err(AppError::Unauthorized(
                "Invalid Authorization header format, expected 'Bearer <token>'".to_string(),
            ));
        };

        let jwt_engine = JwtEngine::from_ref(state);
        let claims = jwt_engine.verify(token)?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Unauthorized("Invalid user UUID in token subject".to_string()))?;

        Ok(AuthenticatedUser {
            user_id,
            email: claims.email.clone(),
            claims,
        })
    }
}
