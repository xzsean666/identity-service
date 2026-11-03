use crate::db::{find_user_by_id, update_user};
use crate::error::AppError;
use crate::models::{UpdateUserDto, User};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct UserService {
    pool: PgPool,
}

impl UserService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<User, AppError> {
        let user = find_user_by_id(&self.pool, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("User not found: {user_id}")))?;

        if user.status != "active" {
            return Err(AppError::Forbidden(format!(
                "User account is {} (not active)",
                user.status
            )));
        }

        Ok(user)
    }

    pub async fn update_user(
        &self,
        user_id: Uuid,
        update: &UpdateUserDto,
    ) -> Result<User, AppError> {
        // Ensure user exists and is active first
        let _ = self.get_user_by_id(user_id).await?;

        update_user(&self.pool, user_id, update).await
    }
}
