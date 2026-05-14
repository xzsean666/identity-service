use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use parking_lot::Mutex;
use uuid::Uuid;

use crate::{
    application::{
        error::AppError,
        identity_binding::IdentityRepository,
        password_change::{PasswordChangeCommand, PasswordChangeRepository},
        readiness::ReadinessDependency,
        session::SessionRepository,
    },
    domain::{
        identity::{ExternalIdentity, NormalizedExternalIdentity},
        session::{RefreshTokenRecord, RefreshTokenStatus, Session, SessionStatus},
        user::InternalUser,
    },
    providers::local_password::{
        LocalCredential, LocalCredentialRepository, LocalCredentialStatus,
    },
};

pub type SharedState = Arc<Mutex<InMemoryState>>;

#[derive(Default)]
pub struct InMemoryState {
    pub users: HashMap<Uuid, InternalUser>,
    pub identities_by_provider_subject: HashMap<(String, String), ExternalIdentity>,
    pub local_credentials_by_username: HashMap<String, LocalCredential>,
    pub sessions: HashMap<Uuid, Session>,
    pub refresh_tokens_by_hash: HashMap<String, RefreshTokenRecord>,
}

#[derive(Clone, Default)]
pub struct InMemoryReadinessCheck;

#[async_trait]
impl ReadinessDependency for InMemoryReadinessCheck {
    fn name(&self) -> &'static str {
        "memory"
    }

    async fn check(&self) -> Result<(), AppError> {
        Ok(())
    }
}

impl InMemoryState {
    pub fn shared() -> SharedState {
        Arc::new(Mutex::new(Self::default()))
    }
}

pub struct InMemoryIdentityRepository {
    state: SharedState,
}

impl InMemoryIdentityRepository {
    pub fn new(state: SharedState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl IdentityRepository for InMemoryIdentityRepository {
    async fn insert_active_user(&self, user: InternalUser) -> Result<(), AppError> {
        let mut state = self.state.lock();
        state.users.insert(user.internal_user_id, user);
        Ok(())
    }

    async fn bound_user(
        &self,
        external_identity: &NormalizedExternalIdentity,
    ) -> Result<Option<InternalUser>, AppError> {
        let key = (
            external_identity.provider_name.clone(),
            external_identity.provider_subject.clone(),
        );
        let state = self.state.lock();

        state
            .identities_by_provider_subject
            .get(&key)
            .map(|binding| {
                state
                    .users
                    .get(&binding.internal_user_id)
                    .cloned()
                    .ok_or_else(|| {
                        AppError::Internal("identity binding points to missing user".to_owned())
                    })
            })
            .transpose()
    }

    async fn bind_new_active_user(
        &self,
        external_identity: NormalizedExternalIdentity,
        now: chrono::DateTime<Utc>,
    ) -> Result<InternalUser, AppError> {
        let key = (
            external_identity.provider_name.clone(),
            external_identity.provider_subject.clone(),
        );
        let user = InternalUser::new_active(now);
        let binding = ExternalIdentity {
            provider_name: external_identity.provider_name,
            provider_subject: external_identity.provider_subject,
            internal_user_id: user.internal_user_id,
            provider_metadata: external_identity.provider_metadata,
            created_at: now,
            updated_at: now,
        };

        let mut state = self.state.lock();
        if state.identities_by_provider_subject.contains_key(&key) {
            return Err(AppError::IdentityConflict);
        }
        state.users.insert(user.internal_user_id, user.clone());
        state.identities_by_provider_subject.insert(key, binding);
        Ok(user)
    }

    async fn bind_existing_user(
        &self,
        internal_user_id: Uuid,
        external_identity: NormalizedExternalIdentity,
        now: chrono::DateTime<Utc>,
    ) -> Result<InternalUser, AppError> {
        let key = (
            external_identity.provider_name.clone(),
            external_identity.provider_subject.clone(),
        );
        let binding = ExternalIdentity {
            provider_name: external_identity.provider_name,
            provider_subject: external_identity.provider_subject,
            internal_user_id,
            provider_metadata: external_identity.provider_metadata,
            created_at: now,
            updated_at: now,
        };

        let mut state = self.state.lock();
        let user = state
            .users
            .get(&internal_user_id)
            .cloned()
            .ok_or(AppError::Unauthorized)?;
        if let Some(existing_binding) = state.identities_by_provider_subject.get(&key) {
            if existing_binding.internal_user_id == internal_user_id {
                return Ok(user);
            }
            return Err(AppError::IdentityConflict);
        }
        state.identities_by_provider_subject.insert(key, binding);
        Ok(user)
    }

    async fn user_by_id(&self, internal_user_id: Uuid) -> Result<InternalUser, AppError> {
        let state = self.state.lock();
        state
            .users
            .get(&internal_user_id)
            .cloned()
            .ok_or(AppError::Unauthorized)
    }

    async fn delete_user(&self, internal_user_id: Uuid) -> Result<(), AppError> {
        let mut state = self.state.lock();
        state.users.remove(&internal_user_id);
        state
            .identities_by_provider_subject
            .retain(|_, identity| identity.internal_user_id != internal_user_id);
        state
            .local_credentials_by_username
            .retain(|_, credential| credential.internal_user_id != internal_user_id);
        state
            .sessions
            .retain(|_, session| session.internal_user_id != internal_user_id);
        state
            .refresh_tokens_by_hash
            .retain(|_, refresh_token| refresh_token.internal_user_id != internal_user_id);
        Ok(())
    }
}

pub struct InMemoryLocalCredentialRepository {
    state: SharedState,
}

impl InMemoryLocalCredentialRepository {
    pub fn new(state: SharedState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl LocalCredentialRepository for InMemoryLocalCredentialRepository {
    async fn create_credential(
        &self,
        normalized_username: &str,
        credential: LocalCredential,
    ) -> Result<LocalCredential, AppError> {
        let mut state = self.state.lock();
        if state
            .local_credentials_by_username
            .contains_key(normalized_username)
        {
            return Err(AppError::IdentityConflict);
        }

        state
            .local_credentials_by_username
            .insert(normalized_username.to_owned(), credential.clone());
        Ok(credential)
    }

    async fn find_by_normalized_username(
        &self,
        normalized_username: &str,
    ) -> Result<Option<LocalCredential>, AppError> {
        let state = self.state.lock();
        Ok(state
            .local_credentials_by_username
            .get(normalized_username)
            .cloned())
    }

    async fn find_by_internal_user_id(
        &self,
        internal_user_id: Uuid,
    ) -> Result<Option<LocalCredential>, AppError> {
        let state = self.state.lock();
        Ok(state
            .local_credentials_by_username
            .values()
            .find(|credential| credential.internal_user_id == internal_user_id)
            .cloned())
    }

    async fn update_for_internal_user_id(
        &self,
        internal_user_id: Uuid,
        credential: LocalCredential,
    ) -> Result<(), AppError> {
        let mut state = self.state.lock();
        let Some(current_key) = state.local_credentials_by_username.iter().find_map(
            |(normalized_username, current)| {
                (current.internal_user_id == internal_user_id).then(|| normalized_username.clone())
            },
        ) else {
            return Err(AppError::InvalidCredentials);
        };

        if current_key != credential.normalized_username
            && state
                .local_credentials_by_username
                .contains_key(&credential.normalized_username)
        {
            return Err(AppError::IdentityConflict);
        }

        state.local_credentials_by_username.remove(&current_key);
        state
            .local_credentials_by_username
            .insert(credential.normalized_username.clone(), credential);
        Ok(())
    }
}

#[derive(Clone)]
pub struct InMemorySessionRepository {
    state: SharedState,
}

impl InMemorySessionRepository {
    pub fn new(state: SharedState) -> Self {
        Self { state }
    }
}

#[derive(Clone)]
pub struct InMemoryPasswordChangeRepository {
    state: SharedState,
}

impl InMemoryPasswordChangeRepository {
    pub fn new(state: SharedState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl PasswordChangeRepository for InMemoryPasswordChangeRepository {
    async fn change_password_and_rotate_refresh_tokens(
        &self,
        command: PasswordChangeCommand,
    ) -> Result<RefreshTokenRecord, AppError> {
        let mut state = self.state.lock();
        let session = state
            .sessions
            .get(&command.current_session_id)
            .cloned()
            .ok_or(AppError::Unauthorized)?;
        if session.internal_user_id != command.internal_user_id
            || session.status != SessionStatus::Active
            || session.expires_at <= command.now
        {
            return Err(AppError::Unauthorized);
        }

        let Some(current_key) = state.local_credentials_by_username.iter().find_map(
            |(normalized_username, current)| {
                (current.internal_user_id == command.internal_user_id)
                    .then(|| normalized_username.clone())
            },
        ) else {
            return Err(AppError::InvalidCredentials);
        };
        let current_credential = state
            .local_credentials_by_username
            .get(&current_key)
            .ok_or(AppError::InvalidCredentials)?;
        if current_credential.status != LocalCredentialStatus::Active
            || current_credential.password_hash
                != command.prepared_password_change.previous_password_hash
        {
            return Err(AppError::InvalidCredentials);
        }

        if current_key
            != command
                .prepared_password_change
                .credential
                .normalized_username
            && state.local_credentials_by_username.contains_key(
                &command
                    .prepared_password_change
                    .credential
                    .normalized_username,
            )
        {
            return Err(AppError::IdentityConflict);
        }

        state.local_credentials_by_username.remove(&current_key);
        state.local_credentials_by_username.insert(
            command
                .prepared_password_change
                .credential
                .normalized_username
                .clone(),
            command.prepared_password_change.credential,
        );

        for refresh_record in state.refresh_tokens_by_hash.values_mut() {
            if refresh_record.internal_user_id == command.internal_user_id {
                refresh_record.status = RefreshTokenStatus::Revoked;
                refresh_record.revoked_at = Some(command.now);
            }
        }
        let new_record = RefreshTokenRecord {
            refresh_token_id: Uuid::new_v4(),
            session_id: command.current_session_id,
            internal_user_id: command.internal_user_id,
            token_family_id: Uuid::new_v4(),
            token_hash: command.new_token_hash,
            status: RefreshTokenStatus::Active,
            issued_at: command.now,
            expires_at: command.now + Duration::seconds(command.refresh_token_lifetime_seconds),
            consumed_at: None,
            revoked_at: None,
        };
        state
            .refresh_tokens_by_hash
            .insert(new_record.token_hash.clone(), new_record.clone());
        Ok(new_record)
    }
}

#[async_trait]
impl SessionRepository for InMemorySessionRepository {
    async fn create_session_with_refresh(
        &self,
        session: Session,
        refresh_token: RefreshTokenRecord,
    ) -> Result<(Session, RefreshTokenRecord), AppError> {
        let mut state = self.state.lock();
        state.sessions.insert(session.session_id, session.clone());
        state
            .refresh_tokens_by_hash
            .insert(refresh_token.token_hash.clone(), refresh_token.clone());
        Ok((session, refresh_token))
    }

    async fn exchange_refresh(
        &self,
        token_hash: &str,
        next_token_hash: String,
        refresh_token_lifetime_seconds: i64,
        now: chrono::DateTime<Utc>,
    ) -> Result<(Session, RefreshTokenRecord), AppError> {
        let mut state = self.state.lock();

        let existing = state
            .refresh_tokens_by_hash
            .get(token_hash)
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
            if let Some(record) = state.refresh_tokens_by_hash.get_mut(token_hash) {
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
            .get_mut(token_hash)
            .ok_or(AppError::TokenInvalid)?;
        existing_record.status = RefreshTokenStatus::Consumed;
        existing_record.consumed_at = Some(now);

        let new_record = RefreshTokenRecord {
            refresh_token_id: Uuid::new_v4(),
            session_id,
            internal_user_id,
            token_family_id: family_id,
            token_hash: next_token_hash,
            status: RefreshTokenStatus::Active,
            issued_at: now,
            expires_at: now + Duration::seconds(refresh_token_lifetime_seconds),
            consumed_at: None,
            revoked_at: None,
        };
        state
            .refresh_tokens_by_hash
            .insert(new_record.token_hash.clone(), new_record.clone());
        Ok((session, new_record))
    }

    async fn revoke_session(
        &self,
        session_id: Uuid,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), AppError> {
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

    async fn rotate_all_user_refresh_families(
        &self,
        internal_user_id: Uuid,
        current_session_id: Uuid,
        new_token_hash: String,
        refresh_token_lifetime_seconds: i64,
        now: chrono::DateTime<Utc>,
    ) -> Result<RefreshTokenRecord, AppError> {
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
        let new_record = RefreshTokenRecord {
            refresh_token_id: Uuid::new_v4(),
            session_id: current_session_id,
            internal_user_id,
            token_family_id: Uuid::new_v4(),
            token_hash: new_token_hash,
            status: RefreshTokenStatus::Active,
            issued_at: now,
            expires_at: now + Duration::seconds(refresh_token_lifetime_seconds),
            consumed_at: None,
            revoked_at: None,
        };
        state
            .refresh_tokens_by_hash
            .insert(new_record.token_hash.clone(), new_record.clone());
        Ok(new_record)
    }

    async fn active_session_by_id(
        &self,
        session_id: Uuid,
        now: chrono::DateTime<Utc>,
    ) -> Result<Session, AppError> {
        let state = self.state.lock();
        let session = state
            .sessions
            .get(&session_id)
            .cloned()
            .ok_or(AppError::Unauthorized)?;
        if session.status != SessionStatus::Active || session.expires_at <= now {
            return Err(AppError::Unauthorized);
        }
        Ok(session)
    }
}
