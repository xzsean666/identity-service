use crate::application::error::AppError;

pub(crate) fn map_sqlx_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.code().as_deref() == Some("23505") {
            return match database_error.constraint() {
                Some("external_identities_pkey")
                | Some("local_credentials_internal_user_id_key")
                | Some("local_credentials_normalized_username_key") => AppError::IdentityConflict,
                _ => AppError::Internal("database unique constraint violation".to_owned()),
            };
        }
    }
    AppError::Internal(error.to_string())
}
