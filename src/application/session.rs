use chrono::{Duration, Utc};
use std::sync::Arc;
use uuid::Uuid;

use async_trait::async_trait;

use crate::{
    application::error::AppError,
    config::{ClientConfig, SessionConfig},
    domain::{
        session::{RefreshTokenRecord, RefreshTokenStatus, Session, SessionStatus},
        token::TokenPair,
        user::InternalUser,
    },
    security::RefreshTokenHasher,
};

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn create_session_with_refresh(
        &self,
        session: Session,
        refresh_token: RefreshTokenRecord,
    ) -> Result<(Session, RefreshTokenRecord), AppError>;

    async fn exchange_refresh(
        &self,
        token_hash: &str,
        next_token_hash: String,
        refresh_token_lifetime_seconds: i64,
        now: chrono::DateTime<Utc>,
    ) -> Result<(Session, RefreshTokenRecord), AppError>;

    async fn revoke_session(
        &self,
        session_id: Uuid,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), AppError>;

    async fn rotate_all_user_refresh_families(
        &self,
        internal_user_id: Uuid,
        current_session_id: Uuid,
        new_token_hash: String,
        refresh_token_lifetime_seconds: i64,
        now: chrono::DateTime<Utc>,
    ) -> Result<RefreshTokenRecord, AppError>;

    async fn active_session_by_id(
        &self,
        session_id: Uuid,
        now: chrono::DateTime<Utc>,
    ) -> Result<Session, AppError>;
}

#[derive(Clone)]
pub struct SessionService {
    repository: Arc<dyn SessionRepository>,
    session_config: SessionConfig,
    client_config: ClientConfig,
    refresh_token_hasher: RefreshTokenHasher,
}

impl SessionService {
    pub fn new(
        repository: Arc<dyn SessionRepository>,
        session_config: SessionConfig,
        client_config: ClientConfig,
        refresh_token_hasher: RefreshTokenHasher,
    ) -> Self {
        Self {
            repository,
            session_config,
            client_config,
            refresh_token_hasher,
        }
    }

    pub async fn create_session(
        &self,
        user: &InternalUser,
        provider_name: &str,
        refresh_token_secret: String,
    ) -> Result<(Session, RefreshTokenRecord), AppError> {
        let now = Utc::now();
        let session = Session {
            session_id: Uuid::new_v4(),
            internal_user_id: user.internal_user_id,
            provider_name: provider_name.to_owned(),
            client_id: self.client_config.client_id.clone(),
            device_metadata: None,
            status: SessionStatus::Active,
            issued_at: now,
            expires_at: now + Duration::seconds(self.session_config.session_lifetime_seconds),
            revoked_at: None,
        };
        let refresh_token = self.create_refresh_record(
            session.session_id,
            session.internal_user_id,
            Uuid::new_v4(),
            refresh_token_secret,
            now,
        )?;

        self.repository
            .create_session_with_refresh(session, refresh_token)
            .await
    }

    pub async fn exchange_refresh_token(
        &self,
        refresh_token: &str,
        next_refresh_token_secret: String,
    ) -> Result<(Session, RefreshTokenRecord), AppError> {
        let token_hash = self.refresh_token_hasher.hash(refresh_token)?;
        let next_token_hash = self.refresh_token_hasher.hash(&next_refresh_token_secret)?;
        let now = Utc::now();
        self.repository
            .exchange_refresh(
                &token_hash,
                next_token_hash,
                self.session_config.refresh_token_lifetime_seconds,
                now,
            )
            .await
    }

    pub async fn revoke_session(&self, session_id: Uuid) -> Result<(), AppError> {
        self.repository.revoke_session(session_id, Utc::now()).await
    }

    pub async fn rotate_all_user_refresh_families(
        &self,
        internal_user_id: Uuid,
        current_session_id: Uuid,
        new_refresh_token_secret: String,
    ) -> Result<RefreshTokenRecord, AppError> {
        let now = Utc::now();
        let new_token_hash = self.refresh_token_hasher.hash(&new_refresh_token_secret)?;
        self.repository
            .rotate_all_user_refresh_families(
                internal_user_id,
                current_session_id,
                new_token_hash,
                self.session_config.refresh_token_lifetime_seconds,
                now,
            )
            .await
    }

    pub async fn session_by_id(&self, session_id: Uuid) -> Result<Session, AppError> {
        self.repository
            .active_session_by_id(session_id, Utc::now())
            .await
    }

    pub fn token_pair(
        &self,
        access_token: String,
        refresh_token: String,
        expires_in: i64,
    ) -> TokenPair {
        TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer",
            expires_in,
        }
    }

    fn create_refresh_record(
        &self,
        session_id: Uuid,
        internal_user_id: Uuid,
        token_family_id: Uuid,
        refresh_token_secret: String,
        now: chrono::DateTime<Utc>,
    ) -> Result<RefreshTokenRecord, AppError> {
        Ok(RefreshTokenRecord {
            refresh_token_id: Uuid::new_v4(),
            session_id,
            internal_user_id,
            token_family_id,
            token_hash: self.refresh_token_hasher.hash(&refresh_token_secret)?,
            status: RefreshTokenStatus::Active,
            issued_at: now,
            expires_at: now + Duration::seconds(self.session_config.refresh_token_lifetime_seconds),
            consumed_at: None,
            revoked_at: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::user::InternalUser, infrastructure::memory::InMemorySessionRepository};

    fn session_service(repository: Arc<dyn SessionRepository>) -> SessionService {
        SessionService::new(
            repository,
            SessionConfig {
                refresh_token_lifetime_seconds: 3600,
                session_lifetime_seconds: 3600,
            },
            ClientConfig {
                client_id: "test-client".to_owned(),
                trusted_origin: None,
            },
            RefreshTokenHasher::new("test-refresh-secret".to_owned()),
        )
    }

    #[tokio::test]
    async fn refresh_exchange_rotates_token_and_detects_reuse() {
        let state = crate::infrastructure::memory::InMemoryState::shared();
        let service = session_service(Arc::new(InMemorySessionRepository::new(state.clone())));
        let user = InternalUser::new_active(Utc::now());

        let (session, first_record) = service
            .create_session(&user, "local_password", "first-refresh".to_owned())
            .await
            .unwrap();
        let (rotated_session, second_record) = service
            .exchange_refresh_token("first-refresh", "second-refresh".to_owned())
            .await
            .unwrap();

        assert_eq!(session.session_id, rotated_session.session_id);
        assert_eq!(first_record.token_family_id, second_record.token_family_id);

        assert!(matches!(
            service
                .exchange_refresh_token("first-refresh", "third-refresh".to_owned())
                .await,
            Err(AppError::RefreshTokenReused)
        ));

        let state = state.lock();
        let reused_record = state
            .refresh_tokens_by_hash
            .values()
            .find(|record| record.refresh_token_id == first_record.refresh_token_id)
            .expect("original refresh token record must exist");
        let revoked_record = state
            .refresh_tokens_by_hash
            .values()
            .find(|record| record.refresh_token_id == second_record.refresh_token_id)
            .expect("rotated refresh token record must exist");

        assert_eq!(reused_record.status, RefreshTokenStatus::Reused);
        assert_eq!(revoked_record.status, RefreshTokenStatus::Revoked);
    }
}
