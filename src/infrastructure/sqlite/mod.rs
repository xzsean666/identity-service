use std::str::FromStr;

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

mod error;
mod health;
mod identity;
mod local_credential;
mod mapping;
mod migration;
mod password_change;
mod session;

pub use health::SqliteReadinessCheck;
pub use identity::SqliteIdentityRepository;
pub use local_credential::SqliteLocalCredentialRepository;
pub use migration::{
    MigrationReport, available_migration_count, revert_migrations, run_pending_migrations,
};
pub use password_change::SqlitePasswordChangeRepository;
pub use session::SqliteSessionRepository;

pub(crate) use error::map_sqlx_error;
pub(crate) use mapping::{
    account_status_to_database, datetime_to_database, local_credential_status_to_database,
    optional_datetime_to_database, optional_json_to_database, refresh_token_status_to_database,
    row_to_internal_user, row_to_local_credential, row_to_refresh_token_record, row_to_session,
    session_status_to_database, uuid_to_database,
};

#[derive(Clone)]
pub struct SqliteState {
    pub pool: SqlitePool,
}

impl SqliteState {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;

        Ok(Self { pool })
    }

    pub async fn health_check(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;

        Ok(())
    }
}
