use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use jsonwebtoken::jwk::JwkSet;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    application::{
        error::AppError, identity_binding::IdentityBindingService,
        password_change::PasswordChangeService, provider_registry::ProviderRegistry,
        session::SessionService, token::TokenService,
    },
    config::SupabaseProviderConfig,
    domain::{
        identity::{
            BindingMode, LOCAL_PASSWORD_PROVIDER, NormalizedExternalIdentity, SUPABASE_PROVIDER,
        },
        session::Session,
        token::TokenPair,
        user::{AccountStatus, InternalUser},
    },
    providers::{
        IdentityProviderAdapter, ProviderVerificationRequest, local_password::LocalCredential,
        local_password::LocalPasswordProvider,
    },
};

const MAX_REFRESH_TOKEN_LENGTH: usize = 512;
const MAX_ACCESS_TOKEN_LENGTH: usize = 8 * 1024;
const MAX_SUPABASE_TOKEN_LENGTH: usize = 16 * 1024;

#[async_trait]
pub trait LocalRegistrationRepository: Send + Sync {
    async fn register_local_user(
        &self,
        user: InternalUser,
        credential: LocalCredential,
        external_identity: NormalizedExternalIdentity,
    ) -> Result<InternalUser, AppError>;
}

#[derive(Clone)]
pub struct AuthService {
    provider_registry: ProviderRegistry,
    identity_binding: IdentityBindingService,
    local_registration_repository: Arc<dyn LocalRegistrationRepository>,
    session_service: SessionService,
    password_change_service: PasswordChangeService,
    token_service: TokenService,
    local_password_provider: Arc<LocalPasswordProvider>,
    supabase_config: SupabaseProviderConfig,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub user: InternalUser,
    pub tokens: TokenPair,
}

impl AuthService {
    pub fn new(
        provider_registry: ProviderRegistry,
        identity_binding: IdentityBindingService,
        local_registration_repository: Arc<dyn LocalRegistrationRepository>,
        session_service: SessionService,
        password_change_service: PasswordChangeService,
        token_service: TokenService,
        local_password_provider: Arc<LocalPasswordProvider>,
        supabase_config: SupabaseProviderConfig,
    ) -> Self {
        Self {
            provider_registry,
            identity_binding,
            local_registration_repository,
            session_service,
            password_change_service,
            token_service,
            local_password_provider,
            supabase_config,
        }
    }

    pub async fn register_with_local_password(
        &self,
        username: String,
        password: String,
    ) -> Result<AuthResponse, AppError> {
        if !self.local_password_provider.descriptor().enabled {
            return Err(AppError::ProviderDisabled);
        }
        let user = InternalUser::new_active(Utc::now());
        let credential = LocalPasswordProvider::prepare_credential_for_user(
            user.internal_user_id,
            &username,
            &password,
        )?;
        let normalized_identity = NormalizedExternalIdentity::local_password(
            credential.credential_id,
            &credential.username,
        );
        let user = self
            .local_registration_repository
            .register_local_user(user, credential, normalized_identity)
            .await?;
        self.issue_platform_tokens(user, LOCAL_PASSWORD_PROVIDER)
            .await
    }

    pub async fn login_with_local_password(
        &self,
        username: String,
        password: String,
    ) -> Result<AuthResponse, AppError> {
        let provider = self.provider_registry.provider(LOCAL_PASSWORD_PROVIDER)?;
        let normalized_identity = provider
            .verify(ProviderVerificationRequest::LocalPassword { username, password })
            .await?;
        let user = self
            .identity_binding
            .resolve_identity(normalized_identity, BindingMode::LoginOnly)
            .await?;
        self.issue_platform_tokens(user, LOCAL_PASSWORD_PROVIDER)
            .await
    }

    pub async fn exchange_supabase_token(
        &self,
        access_token: String,
    ) -> Result<AuthResponse, AppError> {
        validate_token_input(&access_token, MAX_SUPABASE_TOKEN_LENGTH)?;
        let provider = self.provider_registry.provider(SUPABASE_PROVIDER)?;
        let normalized_identity = provider
            .verify(ProviderVerificationRequest::SupabaseToken { access_token })
            .await?;
        let binding_mode = if self.supabase_config.auto_provision_enabled {
            BindingMode::RegisterOrLogin
        } else {
            BindingMode::LoginOnly
        };
        let user = self
            .identity_binding
            .resolve_identity(normalized_identity, binding_mode)
            .await?;
        self.issue_platform_tokens(user, SUPABASE_PROVIDER).await
    }

    pub async fn refresh(&self, refresh_token: String) -> Result<TokenPair, AppError> {
        validate_token_input(&refresh_token, MAX_REFRESH_TOKEN_LENGTH)?;
        let next_refresh_token = self.token_service.generate_refresh_token_secret();
        let (session, _refresh_record) = self
            .session_service
            .exchange_refresh_token(&refresh_token, next_refresh_token.clone())
            .await?;
        if let Err(error) = self.ensure_active_user(session.internal_user_id).await {
            if matches!(error, AppError::AccountDisabled) {
                let _ = self
                    .session_service
                    .revoke_session(session.session_id)
                    .await;
            }
            return Err(error);
        }
        let (access_token, expires_in) = self.token_service.issue_access_token(&session)?;
        Ok(self
            .session_service
            .token_pair(access_token, next_refresh_token, expires_in))
    }

    pub async fn change_local_password(
        &self,
        access_token: &str,
        current_password: String,
        new_password: String,
    ) -> Result<TokenPair, AppError> {
        self.provider_registry.provider(LOCAL_PASSWORD_PROVIDER)?;
        validate_token_input(access_token, MAX_ACCESS_TOKEN_LENGTH)?;
        let claims = self.token_service.verify_access_token(access_token)?;
        let session = self.session_service.session_by_id(claims.sid).await?;
        self.ensure_claims_match_session(claims.sub, &session)?;
        self.ensure_active_user(claims.sub).await?;
        let prepared_password_change = self
            .local_password_provider
            .prepare_password_change(claims.sub, &current_password, &new_password)
            .await?;
        let next_refresh_token = self.token_service.generate_refresh_token_secret();
        let refresh_record = self
            .password_change_service
            .change_password_and_rotate_refresh_tokens(
                claims.sub,
                claims.sid,
                prepared_password_change,
                &next_refresh_token,
            )
            .await?;
        let session = self
            .session_service
            .session_by_id(refresh_record.session_id)
            .await?;
        let (access_token, expires_in) = self.token_service.issue_access_token(&session)?;
        Ok(self
            .session_service
            .token_pair(access_token, next_refresh_token, expires_in))
    }

    pub async fn logout(&self, access_token: &str) -> Result<(), AppError> {
        validate_token_input(access_token, MAX_ACCESS_TOKEN_LENGTH)?;
        let claims = self.token_service.verify_access_token(access_token)?;
        let session = self.session_service.session_by_id(claims.sid).await?;
        self.ensure_claims_match_session(claims.sub, &session)?;
        self.session_service.revoke_session(claims.sid).await
    }

    pub async fn current_user(&self, access_token: &str) -> Result<InternalUser, AppError> {
        validate_token_input(access_token, MAX_ACCESS_TOKEN_LENGTH)?;
        let claims = self.token_service.verify_access_token(access_token)?;
        let session = self.session_service.session_by_id(claims.sid).await?;
        self.ensure_claims_match_session(claims.sub, &session)?;
        self.ensure_active_user(claims.sub).await
    }

    pub fn public_jwks(&self) -> Result<JwkSet, AppError> {
        self.token_service.public_jwks()
    }

    async fn issue_platform_tokens(
        &self,
        user: InternalUser,
        provider_name: &str,
    ) -> Result<AuthResponse, AppError> {
        if user.account_status != AccountStatus::Active {
            return Err(AppError::AccountDisabled);
        }
        let refresh_token = self.token_service.generate_refresh_token_secret();
        let (session, _refresh_record) = self
            .session_service
            .create_session(&user, provider_name, refresh_token.clone())
            .await?;
        let (access_token, expires_in) = self.token_service.issue_access_token(&session)?;
        let tokens = self
            .session_service
            .token_pair(access_token, refresh_token, expires_in);
        Ok(AuthResponse { user, tokens })
    }

    async fn ensure_active_user(&self, internal_user_id: Uuid) -> Result<InternalUser, AppError> {
        let user = self.identity_binding.user_by_id(internal_user_id).await?;
        if user.account_status != AccountStatus::Active {
            return Err(AppError::AccountDisabled);
        }
        Ok(user)
    }

    fn ensure_claims_match_session(
        &self,
        subject: Uuid,
        session: &Session,
    ) -> Result<(), AppError> {
        if session.internal_user_id != subject {
            return Err(AppError::Unauthorized);
        }
        Ok(())
    }
}

fn validate_token_input(value: &str, max_len: usize) -> Result<(), AppError> {
    if value.trim().is_empty() || value.len() > max_len || value.chars().any(char::is_control) {
        return Err(AppError::ValidationFailed);
    }
    Ok(())
}
