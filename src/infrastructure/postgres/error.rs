use crate::application::error::AppError;

pub(crate) fn map_sqlx_error(error: sqlx::Error) -> AppError {
    if is_unique_violation(&error) {
        return AppError::IdentityConflict;
    }
    AppError::Internal(error.to_string())
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("23505")
    )
}
