use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    application::error::AppError,
    config::{ClientConfig, SessionConfig},
    domain::{
        session::{RefreshTokenRecord, RefreshTokenStatus, Session, SessionStatus},
        token::TokenPair,
        user::InternalUser,
    },
    infrastructure::memory::SharedState,
    security::RefreshTokenHasher,
};

#[derive(Clone)]
pub struct SessionService {
    state: SharedState,
    session_config: SessionConfig,
    client_config: ClientConfig,
    refresh_token_hasher: RefreshTokenHasher,
}

impl SessionService {
    pub fn new(
        state: SharedState,
        session_config: SessionConfig,
        client_config: ClientConfig,
        refresh_token_hasher: RefreshTokenHasher,
    ) -> Self {
        Self {
            state,
            session_config,
            client_config,
            refresh_token_hasher,
        }
    }

    pub fn create_session(
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

        let mut state = self.state.lock();
        state.sessions.insert(session.session_id, session.clone());
        state
            .refresh_tokens_by_hash
            .insert(refresh_token.token_hash.clone(), refresh_token.clone());
        Ok((session, refresh_token))
    }

    pub fn exchange_refresh_token(
        &self,
        refresh_token: &str,
        next_refresh_token_secret: String,
    ) -> Result<(Session, RefreshTokenRecord), AppError> {
        let token_hash = self.refresh_token_hasher.hash(refresh_token)?;
        let now = Utc::now();
        let mut state = self.state.lock();

        let existing = state
            .refresh_tokens_by_hash
            .get(&token_hash)
            .cloned()
            .ok_or(AppError::TokenInvalid)?;

        if existing.status == RefreshTokenStatus::Consumed {
            let family_id = existing.token_family_id;
            for record in state.refresh_tokens_by_hash.values_mut() {
                if record.token_family_id == family_id {
                    if record.refresh_token_id == existing.refresh_token_id {
                        record.status = RefreshTokenStatus::Reused;
                    } else {
                        record.status = RefreshTokenStatus::Revoked;
                    }
                    record.revoked_at = Some(now);
                }
            }
            return Err(AppError::RefreshTokenReused);
        }

        if existing.status != RefreshTokenStatus::Active {
            return Err(AppError::TokenInvalid);
        }
        if existing.expires_at <= now {
            if let Some(record) = state.refresh_tokens_by_hash.get_mut(&token_hash) {
                record.status = RefreshTokenStatus::Expired;
            }
            return Err(AppError::TokenInvalid);
        }

        let session_id = existing.session_id;
        let family_id = existing.token_family_id;
        let internal_user_id = existing.internal_user_id;

        let session = state
            .sessions
            .get(&session_id)
            .cloned()
            .ok_or(AppError::TokenInvalid)?;
        if session.status != SessionStatus::Active || session.expires_at <= now {
            return Err(AppError::TokenInvalid);
        }

        let existing_record = state
            .refresh_tokens_by_hash
            .get_mut(&token_hash)
            .ok_or(AppError::TokenInvalid)?;
        existing_record.status = RefreshTokenStatus::Consumed;
        existing_record.consumed_at = Some(now);

        let new_record = self.create_refresh_record(
            session_id,
            internal_user_id,
            family_id,
            next_refresh_token_secret,
            now,
        )?;
        state
            .refresh_tokens_by_hash
            .insert(new_record.token_hash.clone(), new_record.clone());
        Ok((session, new_record))
    }

    pub fn revoke_session(&self, session_id: Uuid) -> Result<(), AppError> {
        let now = Utc::now();
        let mut state = self.state.lock();
        {
            let session = state
                .sessions
                .get_mut(&session_id)
                .ok_or(AppError::Unauthorized)?;
            session.status = SessionStatus::Revoked;
            session.revoked_at = Some(now);
        }
        for refresh_record in state.refresh_tokens_by_hash.values_mut() {
            if refresh_record.session_id == session_id {
                refresh_record.status = RefreshTokenStatus::Revoked;
                refresh_record.revoked_at = Some(now);
            }
        }
        Ok(())
    }

    pub fn rotate_all_user_refresh_families(
        &self,
        internal_user_id: Uuid,
        current_session_id: Uuid,
        new_refresh_token_secret: String,
    ) -> Result<RefreshTokenRecord, AppError> {
        let now = Utc::now();
        let mut state = self.state.lock();
        let session = state
            .sessions
            .get(&current_session_id)
            .cloned()
            .ok_or(AppError::Unauthorized)?;
        if session.internal_user_id != internal_user_id
            || session.status != SessionStatus::Active
            || session.expires_at <= now
        {
            return Err(AppError::Unauthorized);
        }

        for refresh_record in state.refresh_tokens_by_hash.values_mut() {
            if refresh_record.internal_user_id == internal_user_id {
                refresh_record.status = RefreshTokenStatus::Revoked;
                refresh_record.revoked_at = Some(now);
            }
        }
        let new_record = self.create_refresh_record(
            current_session_id,
            internal_user_id,
            Uuid::new_v4(),
            new_refresh_token_secret,
            now,
        )?;
        state
            .refresh_tokens_by_hash
            .insert(new_record.token_hash.clone(), new_record.clone());
        Ok(new_record)
    }

    pub fn session_by_id(&self, session_id: Uuid) -> Result<Session, AppError> {
        let state = self.state.lock();
        let session = state
            .sessions
            .get(&session_id)
            .cloned()
            .ok_or(AppError::Unauthorized)?;
        if session.status != SessionStatus::Active || session.expires_at <= Utc::now() {
            return Err(AppError::Unauthorized);
        }
        Ok(session)
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
    use crate::domain::user::InternalUser;

    fn session_service(state: SharedState) -> SessionService {
        SessionService::new(
            state,
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

    #[test]
    fn refresh_exchange_rotates_token_and_detects_reuse() {
        let state = crate::infrastructure::memory::InMemoryState::shared();
        let service = session_service(state.clone());
        let user = InternalUser::new_active(Utc::now());

        let (session, first_record) = service
            .create_session(&user, "local_password", "first-refresh".to_owned())
            .unwrap();
        let (rotated_session, second_record) = service
            .exchange_refresh_token("first-refresh", "second-refresh".to_owned())
            .unwrap();

        assert_eq!(session.session_id, rotated_session.session_id);
        assert_eq!(first_record.token_family_id, second_record.token_family_id);

        assert!(matches!(
            service.exchange_refresh_token("first-refresh", "third-refresh".to_owned()),
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
