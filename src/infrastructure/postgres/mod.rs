use sqlx::{PgPool, postgres::PgPoolOptions};

mod error;
mod identity;
mod local_credential;
mod mapping;
mod session;

pub use identity::PostgresIdentityRepository;
pub use local_credential::PostgresLocalCredentialRepository;
pub use session::PostgresSessionRepository;

pub(crate) use error::map_sqlx_error;
pub(crate) use mapping::{
    account_status_to_database, local_credential_status_to_database,
    refresh_token_status_to_database, row_to_internal_user, row_to_local_credential,
    row_to_refresh_token_record, row_to_session, session_status_to_database,
};

#[derive(Clone)]
pub struct PostgresState {
    pub pool: PgPool,
}

impl PostgresState {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new().connect(database_url).await?;

        Ok(Self { pool })
    }

    pub async fn health_check(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;

        Ok(())
    }
}
