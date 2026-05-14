use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("validation failed")]
    ValidationFailed,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("provider disabled")]
    ProviderDisabled,
    #[error("provider verification failed")]
    ProviderVerificationFailed,
    #[error("identity conflict")]
    IdentityConflict,
    #[error("unauthorized")]
    Unauthorized,
    #[error("token invalid")]
    TokenInvalid,
    #[error("refresh token reused")]
    RefreshTokenReused,
    #[error("account disabled")]
    AccountDisabled,
    #[error("not found")]
    NotFound,
    #[error("dependency unavailable: {0}")]
    DependencyUnavailable(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error_code: &'static str,
    pub message: String,
    pub retryable: bool,
}

impl AppError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::ValidationFailed => "validation_failed",
            Self::InvalidCredentials => "invalid_credentials",
            Self::ProviderDisabled => "provider_disabled",
            Self::ProviderVerificationFailed => "provider_verification_failed",
            Self::IdentityConflict => "identity_conflict",
            Self::Unauthorized => "unauthorized",
            Self::TokenInvalid => "token_invalid",
            Self::RefreshTokenReused => "refresh_token_reused",
            Self::AccountDisabled => "account_disabled",
            Self::NotFound => "not_found",
            Self::DependencyUnavailable(_) => "dependency_unavailable",
            Self::Internal(_) => "internal_error",
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::ValidationFailed => StatusCode::BAD_REQUEST,
            Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Self::ProviderDisabled => StatusCode::FORBIDDEN,
            Self::ProviderVerificationFailed => StatusCode::UNAUTHORIZED,
            Self::IdentityConflict => StatusCode::CONFLICT,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::TokenInvalid => StatusCode::UNAUTHORIZED,
            Self::RefreshTokenReused => StatusCode::UNAUTHORIZED,
            Self::AccountDisabled => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::DependencyUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = self.status_code();
        let body = ErrorResponse {
            error_code: self.error_code(),
            message: self.to_string(),
            retryable: false,
        };
        (status, Json(body)).into_response()
    }
}
