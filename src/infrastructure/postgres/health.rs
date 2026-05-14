use async_trait::async_trait;
use sqlx::PgPool;

use crate::application::{error::AppError, readiness::ReadinessDependency};

#[derive(Clone)]
pub struct PostgresReadinessCheck {
    pool: PgPool,
}

impl PostgresReadinessCheck {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReadinessDependency for PostgresReadinessCheck {
    fn name(&self) -> &'static str {
        "postgres"
    }

    async fn check(&self) -> Result<(), AppError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|error| {
                AppError::DependencyUnavailable(format!("postgres readiness check failed: {error}"))
            })?;

        Ok(())
    }
}
