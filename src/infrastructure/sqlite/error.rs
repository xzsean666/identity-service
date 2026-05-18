use crate::application::error::AppError;

pub(crate) fn map_sqlx_error(error: sqlx::Error) -> AppError {
    if is_unique_violation(&error) {
        return if is_identity_unique_violation(&error) {
            AppError::IdentityConflict
        } else {
            AppError::Internal("database unique constraint violation".to_owned())
        };
    }
    AppError::Internal(error.to_string())
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(database_error)
            if matches!(database_error.code().as_deref(), Some("1555" | "2067"))
                || database_error.message().contains("UNIQUE constraint failed")
    )
}

fn is_identity_unique_violation(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(database_error)
            if database_error.message().contains("external_identities")
                || database_error.message().contains("local_credentials.internal_user_id")
                || database_error.message().contains("local_credentials.normalized_username")
    )
}
