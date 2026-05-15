use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::{Row, sqlite::SqliteRow};
use uuid::Uuid;

use crate::{
    application::error::AppError,
    domain::{
        session::{RefreshTokenRecord, RefreshTokenStatus, Session, SessionStatus},
        user::{AccountStatus, InternalUser},
    },
    providers::local_password::{LocalCredential, LocalCredentialStatus},
};

use super::map_sqlx_error;

pub(crate) fn row_to_internal_user(row: SqliteRow) -> Result<InternalUser, AppError> {
    let status: String = row.try_get("account_status").map_err(map_sqlx_error)?;
    Ok(InternalUser {
        internal_user_id: uuid_from_database(
            row.try_get("internal_user_id").map_err(map_sqlx_error)?,
        )?,
        account_status: account_status_from_database(&status)?,
        created_at: datetime_from_database(row.try_get("created_at").map_err(map_sqlx_error)?)?,
        updated_at: datetime_from_database(row.try_get("updated_at").map_err(map_sqlx_error)?)?,
    })
}

pub(crate) fn row_to_local_credential(row: SqliteRow) -> Result<LocalCredential, AppError> {
    let status: String = row.try_get("status").map_err(map_sqlx_error)?;
    Ok(LocalCredential {
        credential_id: uuid_from_database(row.try_get("credential_id").map_err(map_sqlx_error)?)?,
        internal_user_id: uuid_from_database(
            row.try_get("internal_user_id").map_err(map_sqlx_error)?,
        )?,
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
        created_at: datetime_from_database(row.try_get("created_at").map_err(map_sqlx_error)?)?,
        updated_at: datetime_from_database(row.try_get("updated_at").map_err(map_sqlx_error)?)?,
    })
}

pub(crate) fn row_to_session(row: SqliteRow) -> Result<Session, AppError> {
    let status: String = row.try_get("status").map_err(map_sqlx_error)?;
    Ok(Session {
        session_id: uuid_from_database(row.try_get("session_id").map_err(map_sqlx_error)?)?,
        internal_user_id: uuid_from_database(
            row.try_get("internal_user_id").map_err(map_sqlx_error)?,
        )?,
        provider_name: row.try_get("provider_name").map_err(map_sqlx_error)?,
        client_id: row.try_get("client_id").map_err(map_sqlx_error)?,
        device_metadata: optional_json_from_database(
            row.try_get("device_metadata").map_err(map_sqlx_error)?,
        )?,
        status: session_status_from_database(&status)?,
        issued_at: datetime_from_database(row.try_get("issued_at").map_err(map_sqlx_error)?)?,
        expires_at: datetime_from_database(row.try_get("expires_at").map_err(map_sqlx_error)?)?,
        revoked_at: optional_datetime_from_database(
            row.try_get("revoked_at").map_err(map_sqlx_error)?,
        )?,
    })
}

pub(crate) fn row_to_refresh_token_record(row: SqliteRow) -> Result<RefreshTokenRecord, AppError> {
    let status: String = row.try_get("status").map_err(map_sqlx_error)?;
    Ok(RefreshTokenRecord {
        refresh_token_id: uuid_from_database(
            row.try_get("refresh_token_id").map_err(map_sqlx_error)?,
        )?,
        session_id: uuid_from_database(row.try_get("session_id").map_err(map_sqlx_error)?)?,
        internal_user_id: uuid_from_database(
            row.try_get("internal_user_id").map_err(map_sqlx_error)?,
        )?,
        token_family_id: uuid_from_database(
            row.try_get("token_family_id").map_err(map_sqlx_error)?,
        )?,
        token_hash: row.try_get("token_hash").map_err(map_sqlx_error)?,
        status: refresh_token_status_from_database(&status)?,
        issued_at: datetime_from_database(row.try_get("issued_at").map_err(map_sqlx_error)?)?,
        expires_at: datetime_from_database(row.try_get("expires_at").map_err(map_sqlx_error)?)?,
        consumed_at: optional_datetime_from_database(
            row.try_get("consumed_at").map_err(map_sqlx_error)?,
        )?,
        revoked_at: optional_datetime_from_database(
            row.try_get("revoked_at").map_err(map_sqlx_error)?,
        )?,
    })
}

pub(crate) fn uuid_to_database(value: Uuid) -> String {
    value.to_string()
}

fn uuid_from_database(value: String) -> Result<Uuid, AppError> {
    Uuid::parse_str(&value).map_err(|error| AppError::Internal(error.to_string()))
}

pub(crate) fn datetime_to_database(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Nanos, true)
}

pub(crate) fn optional_datetime_to_database(value: Option<DateTime<Utc>>) -> Option<String> {
    value.map(datetime_to_database)
}

pub(crate) fn datetime_from_database(value: String) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(&value)
        .map(|datetime| datetime.with_timezone(&Utc))
        .map_err(|error| AppError::Internal(error.to_string()))
}

pub(crate) fn optional_datetime_from_database(
    value: Option<String>,
) -> Result<Option<DateTime<Utc>>, AppError> {
    value.map(datetime_from_database).transpose()
}

pub(crate) fn optional_json_to_database(value: &Option<serde_json::Value>) -> Option<String> {
    value.as_ref().map(serde_json::Value::to_string)
}

fn optional_json_from_database(
    value: Option<String>,
) -> Result<Option<serde_json::Value>, AppError> {
    value
        .map(|value| {
            serde_json::from_str(&value).map_err(|error| AppError::Internal(error.to_string()))
        })
        .transpose()
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
