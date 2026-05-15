use sqlx::{SqlitePool, migrate::Migrator};

use crate::application::error::AppError;

use super::{SqliteState, map_sqlx_error};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations/sqlite");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub available_up_migrations: usize,
}

pub fn available_migration_count() -> usize {
    MIGRATOR
        .iter()
        .filter(|migration| migration.migration_type.is_up_migration())
        .count()
}

pub async fn run_pending_migrations(database_url: &str) -> Result<MigrationReport, AppError> {
    let state = SqliteState::connect(database_url)
        .await
        .map_err(map_sqlx_error)?;
    run_pending_migrations_on_pool(&state.pool).await
}

pub async fn revert_migrations(
    database_url: &str,
    target_version: i64,
) -> Result<MigrationReport, AppError> {
    let state = SqliteState::connect(database_url)
        .await
        .map_err(map_sqlx_error)?;
    revert_migrations_on_pool(&state.pool, target_version).await
}

async fn run_pending_migrations_on_pool(pool: &SqlitePool) -> Result<MigrationReport, AppError> {
    MIGRATOR
        .run(pool)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

    Ok(MigrationReport {
        available_up_migrations: available_migration_count(),
    })
}

async fn revert_migrations_on_pool(
    pool: &SqlitePool,
    target_version: i64,
) -> Result<MigrationReport, AppError> {
    MIGRATOR
        .undo(pool, target_version)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;

    Ok(MigrationReport {
        available_up_migrations: available_migration_count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mvp_migration_is_registered() {
        assert_eq!(available_migration_count(), 1);
    }
}
