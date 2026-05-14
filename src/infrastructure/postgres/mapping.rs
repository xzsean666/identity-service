use sqlx::{Row, postgres::PgRow};

use crate::{
    application::error::AppError,
    domain::{
        session::{RefreshTokenRecord, RefreshTokenStatus, Session, SessionStatus},
        user::{AccountStatus, InternalUser},
    },
    providers::local_password::{LocalCredential, LocalCredentialStatus},
};

use super::map_sqlx_error;

pub(crate) fn row_to_internal_user(row: PgRow) -> Result<InternalUser, AppError> {
    let status: String = row.try_get("account_status").map_err(map_sqlx_error)?;
    Ok(InternalUser {
        internal_user_id: row.try_get("internal_user_id").map_err(map_sqlx_error)?,
        account_status: account_status_from_database(&status)?,
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
        updated_at: row.try_get("updated_at").map_err(map_sqlx_error)?,
    })
}

pub(crate) fn row_to_local_credential(row: PgRow) -> Result<LocalCredential, AppError> {
    let status: String = row.try_get("status").map_err(map_sqlx_error)?;
    Ok(LocalCredential {
        credential_id: row.try_get("credential_id").map_err(map_sqlx_error)?,
        internal_user_id: row.try_get("internal_user_id").map_err(map_sqlx_error)?,
        username: row.try_get("username").map_err(map_sqlx_error)?,
        normalized_username: row.try_get("normalized_username").map_err(map_sqlx_error)?,
        password_hash: row.try_get("password_hash").map_err(map_sqlx_error)?,
        password_hash_algorithm: row
            .try_get("password_hash_algorithm")
            .map_err(map_sqlx_error)?,
        password_hash_parameters: row
            .try_get("password_hash_parameters")
            .map_err(map_sqlx_error)?,
        status: local_credential_status_from_database(&status)?,
        created_at: row.try_get("created_at").map_err(map_sqlx_error)?,
        updated_at: row.try_get("updated_at").map_err(map_sqlx_error)?,
    })
}

pub(crate) fn row_to_session(row: PgRow) -> Result<Session, AppError> {
    let status: String = row.try_get("status").map_err(map_sqlx_error)?;
    Ok(Session {
        session_id: row.try_get("session_id").map_err(map_sqlx_error)?,
        internal_user_id: row.try_get("internal_user_id").map_err(map_sqlx_error)?,
        provider_name: row.try_get("provider_name").map_err(map_sqlx_error)?,
        client_id: row.try_get("client_id").map_err(map_sqlx_error)?,
        device_metadata: row.try_get("device_metadata").map_err(map_sqlx_error)?,
        status: session_status_from_database(&status)?,
        issued_at: row.try_get("issued_at").map_err(map_sqlx_error)?,
        expires_at: row.try_get("expires_at").map_err(map_sqlx_error)?,
        revoked_at: row.try_get("revoked_at").map_err(map_sqlx_error)?,
    })
}

pub(crate) fn row_to_refresh_token_record(row: PgRow) -> Result<RefreshTokenRecord, AppError> {
    let status: String = row.try_get("status").map_err(map_sqlx_error)?;
    Ok(RefreshTokenRecord {
        refresh_token_id: row.try_get("refresh_token_id").map_err(map_sqlx_error)?,
        session_id: row.try_get("session_id").map_err(map_sqlx_error)?,
        internal_user_id: row.try_get("internal_user_id").map_err(map_sqlx_error)?,
        token_family_id: row.try_get("token_family_id").map_err(map_sqlx_error)?,
        token_hash: row.try_get("token_hash").map_err(map_sqlx_error)?,
        status: refresh_token_status_from_database(&status)?,
        issued_at: row.try_get("issued_at").map_err(map_sqlx_error)?,
        expires_at: row.try_get("expires_at").map_err(map_sqlx_error)?,
        consumed_at: row.try_get("consumed_at").map_err(map_sqlx_error)?,
        revoked_at: row.try_get("revoked_at").map_err(map_sqlx_error)?,
    })
}

pub(crate) fn account_status_to_database(status: &AccountStatus) -> &'static str {
    match status {
        AccountStatus::Active => "active",
        AccountStatus::Disabled => "disabled",
    }
}

fn account_status_from_database(status: &str) -> Result<AccountStatus, AppError> {
    match status {
        "active" => Ok(AccountStatus::Active),
        "disabled" => Ok(AccountStatus::Disabled),
        _ => Err(AppError::Internal(format!(
            "unknown account status from database: {status}"
        ))),
    }
}

pub(crate) fn local_credential_status_to_database(status: &LocalCredentialStatus) -> &'static str {
    match status {
        LocalCredentialStatus::Active => "active",
        LocalCredentialStatus::Disabled => "disabled",
    }
}

fn local_credential_status_from_database(status: &str) -> Result<LocalCredentialStatus, AppError> {
    match status {
        "active" => Ok(LocalCredentialStatus::Active),
        "disabled" => Ok(LocalCredentialStatus::Disabled),
        _ => Err(AppError::Internal(format!(
            "unknown local credential status from database: {status}"
        ))),
    }
}

pub(crate) fn session_status_to_database(status: &SessionStatus) -> &'static str {
    match status {
        SessionStatus::Active => "active",
        SessionStatus::Revoked => "revoked",
        SessionStatus::Expired => "expired",
    }
}

fn session_status_from_database(status: &str) -> Result<SessionStatus, AppError> {
    match status {
        "active" => Ok(SessionStatus::Active),
        "revoked" => Ok(SessionStatus::Revoked),
        "expired" => Ok(SessionStatus::Expired),
        _ => Err(AppError::Internal(format!(
            "unknown session status from database: {status}"
        ))),
    }
}

pub(crate) fn refresh_token_status_to_database(status: &RefreshTokenStatus) -> &'static str {
    match status {
        RefreshTokenStatus::Active => "active",
        RefreshTokenStatus::Consumed => "consumed",
        RefreshTokenStatus::Revoked => "revoked",
        RefreshTokenStatus::Reused => "reused",
        RefreshTokenStatus::Expired => "expired",
    }
}

fn refresh_token_status_from_database(status: &str) -> Result<RefreshTokenStatus, AppError> {
    match status {
        "active" => Ok(RefreshTokenStatus::Active),
        "consumed" => Ok(RefreshTokenStatus::Consumed),
        "revoked" => Ok(RefreshTokenStatus::Revoked),
        "reused" => Ok(RefreshTokenStatus::Reused),
        "expired" => Ok(RefreshTokenStatus::Expired),
        _ => Err(AppError::Internal(format!(
            "unknown refresh token status from database: {status}"
        ))),
    }
}
