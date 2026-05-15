use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::{application::error::AppError, application::readiness::ReadinessDependency};

#[derive(Clone)]
pub struct SqliteReadinessCheck {
    pool: SqlitePool,
}

impl SqliteReadinessCheck {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReadinessDependency for SqliteReadinessCheck {
    fn name(&self) -> &'static str {
        "sqlite"
    }

    async fn check(&self) -> Result<(), AppError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(|error| {
                AppError::DependencyUnavailable(format!("sqlite readiness check failed: {error}"))
            })?;

        Ok(())
    }
}
