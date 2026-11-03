use crate::error::AppError;
use crate::models::{ExternalIdentity, Identity, RefreshToken, UpdateUserDto, User};
use chrono::{DateTime, Utc};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use uuid::Uuid;

pub async fn init_pool(database_url: &str) -> Result<PgPool, AppError> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .connect(database_url)
        .await
        .map_err(|e| AppError::Config(format!("Failed to connect to database: {e}")))?;

    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), AppError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| AppError::Config(format!("Database migration failed: {e}")))?;

    Ok(())
}

pub async fn find_user_by_id(pool: &PgPool, user_id: Uuid) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, email, name, avatar_url, status, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

pub async fn update_user(
    pool: &PgPool,
    user_id: Uuid,
    update: &UpdateUserDto,
) -> Result<User, AppError> {
    let user = sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET name = COALESCE($2, name),
            avatar_url = COALESCE($3, avatar_url),
            updated_at = now()
        WHERE id = $1
        RETURNING id, email, name, avatar_url, status, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(&update.name)
    .bind(&update.avatar_url)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("User not found: {}", user_id)))?;

    Ok(user)
}

pub async fn find_identity(
    pool: &PgPool,
    provider: &str,
    provider_user_id: &str,
) -> Result<Option<Identity>, AppError> {
    let identity = sqlx::query_as::<_, Identity>(
        r#"
        SELECT id, user_id, provider, provider_user_id, created_at
        FROM identities
        WHERE provider = $1 AND provider_user_id = $2
        "#,
    )
    .bind(provider)
    .bind(provider_user_id)
    .fetch_optional(pool)
    .await?;

    Ok(identity)
}

pub async fn create_user_with_identity(
    pool: &PgPool,
    ext: &ExternalIdentity,
) -> Result<(User, Identity), AppError> {
    let mut tx = pool.begin().await?;

    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, name, avatar_url)
        VALUES ($1, $2, $3)
        RETURNING id, email, name, avatar_url, status, created_at, updated_at
        "#,
    )
    .bind(&ext.email)
    .bind(&ext.name)
    .bind(&ext.avatar_url)
    .fetch_one(&mut *tx)
    .await?;

    let identity = sqlx::query_as::<_, Identity>(
        r#"
        INSERT INTO identities (user_id, provider, provider_user_id)
        VALUES ($1, $2, $3)
        RETURNING id, user_id, provider, provider_user_id, created_at
        "#,
    )
    .bind(user.id)
    .bind(&ext.provider)
    .bind(&ext.provider_user_id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((user, identity))
}

pub async fn save_refresh_token(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<RefreshToken, AppError> {
    let rt = sqlx::query_as::<_, RefreshToken>(
        r#"
        INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
        VALUES ($1, $2, $3)
        RETURNING id, user_id, token_hash, expires_at, revoked, created_at
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;

    Ok(rt)
}

pub async fn find_refresh_token(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<RefreshToken>, AppError> {
    let rt = sqlx::query_as::<_, RefreshToken>(
        r#"
        SELECT id, user_id, token_hash, expires_at, revoked, created_at
        FROM refresh_tokens
        WHERE token_hash = $1
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?;

    Ok(rt)
}

pub async fn link_identity_to_user(
    pool: &PgPool,
    user_id: Uuid,
    ext: &ExternalIdentity,
) -> Result<Identity, AppError> {
    if let Some(existing) = find_identity(pool, &ext.provider, &ext.provider_user_id).await? {
        if existing.user_id == user_id {
            return Ok(existing);
        }
        return Err(AppError::Conflict(format!(
            "Identity '{}:{}' is already linked to another user",
            ext.provider, ext.provider_user_id
        )));
    }

    let identity = sqlx::query_as::<_, Identity>(
        r#"
        INSERT INTO identities (user_id, provider, provider_user_id)
        VALUES ($1, $2, $3)
        RETURNING id, user_id, provider, provider_user_id, created_at
        "#,
    )
    .bind(user_id)
    .bind(&ext.provider)
    .bind(&ext.provider_user_id)
    .fetch_one(pool)
    .await?;

    Ok(identity)
}

pub async fn find_identities_by_user_id(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<Identity>, AppError> {
    let list = sqlx::query_as::<_, Identity>(
        r#"
        SELECT id, user_id, provider, provider_user_id, created_at
        FROM identities
        WHERE user_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(list)
}

pub async fn revoke_refresh_token(pool: &PgPool, token_hash: &str) -> Result<bool, AppError> {
    let result = sqlx::query(
        r#"
        UPDATE refresh_tokens
        SET revoked = true
        WHERE token_hash = $1
        "#,
    )
    .bind(token_hash)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn revoke_all_user_refresh_tokens(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<u64, AppError> {
    let result = sqlx::query(
        r#"
        UPDATE refresh_tokens
        SET revoked = true
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
