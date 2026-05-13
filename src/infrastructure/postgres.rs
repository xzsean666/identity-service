use sqlx::{PgPool, postgres::PgPoolOptions};

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
