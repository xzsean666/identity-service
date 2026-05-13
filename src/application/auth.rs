use std::sync::Arc;

use serde::Serialize;

use crate::{
    application::{
        error::AppError, identity_binding::IdentityBindingService,
        provider_registry::ProviderRegistry, session::SessionService, token::TokenService,
    },
    config::SupabaseProviderConfig,
    domain::{
        identity::{BindingMode, LOCAL_PASSWORD_PROVIDER, SUPABASE_PROVIDER},
        token::TokenPair,
        user::InternalUser,
    },
    providers::{
        IdentityProviderAdapter, ProviderVerificationRequest, local_password::LocalPasswordProvider,
    },
};

#[derive(Clone)]
pub struct AuthService {
    provider_registry: ProviderRegistry,
    identity_binding: IdentityBindingService,
    session_service: SessionService,
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
        session_service: SessionService,
        token_service: TokenService,
        local_password_provider: Arc<LocalPasswordProvider>,
        supabase_config: SupabaseProviderConfig,
    ) -> Self {
        Self {
            provider_registry,
            identity_binding,
            session_service,
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
        let user = self.identity_binding.create_active_user();
        let credential = self.local_password_provider.create_credential_for_user(
            user.internal_user_id,
            &username,
            &password,
        )?;
        let normalized_identity =
            crate::domain::identity::NormalizedExternalIdentity::local_password(
                credential.credential_id,
                &credential.username,
            );
        let user = self.identity_binding.resolve_identity(
            normalized_identity,
            BindingMode::LinkToExisting(user.internal_user_id),
        )?;
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
            .resolve_identity(normalized_identity, BindingMode::LoginOnly)?;
        self.issue_platform_tokens(user, LOCAL_PASSWORD_PROVIDER)
            .await
    }

    pub async fn exchange_supabase_token(
        &self,
        access_token: String,
    ) -> Result<AuthResponse, AppError> {
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
            .resolve_identity(normalized_identity, binding_mode)?;
        self.issue_platform_tokens(user, SUPABASE_PROVIDER).await
    }

    pub async fn refresh(&self, refresh_token: String) -> Result<TokenPair, AppError> {
        let next_refresh_token = self.token_service.generate_refresh_token_secret();
        let (session, _refresh_record) = self
            .session_service
            .exchange_refresh_token(&refresh_token, next_refresh_token.clone())?;
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
        let claims = self.token_service.verify_access_token(access_token)?;
        let session = self.session_service.session_by_id(claims.sid)?;
        if session.internal_user_id != claims.sub {
            return Err(AppError::Unauthorized);
        }
        self.local_password_provider.change_password(
            claims.sub,
            &current_password,
            &new_password,
        )?;
        let next_refresh_token = self.token_service.generate_refresh_token_secret();
        let refresh_record = self.session_service.rotate_all_user_refresh_families(
            claims.sub,
            claims.sid,
            next_refresh_token.clone(),
        )?;
        let session = self
            .session_service
            .session_by_id(refresh_record.session_id)?;
        let (access_token, expires_in) = self.token_service.issue_access_token(&session)?;
        Ok(self
            .session_service
            .token_pair(access_token, next_refresh_token, expires_in))
    }

    pub async fn logout(&self, access_token: &str) -> Result<(), AppError> {
        let claims = self.token_service.verify_access_token(access_token)?;
        self.session_service.revoke_session(claims.sid)
    }

    pub async fn current_user(&self, access_token: &str) -> Result<InternalUser, AppError> {
        let claims = self.token_service.verify_access_token(access_token)?;
        let _session = self.session_service.session_by_id(claims.sid)?;
        self.identity_binding.user_by_id(claims.sub)
    }

    async fn issue_platform_tokens(
        &self,
        user: InternalUser,
        provider_name: &str,
    ) -> Result<AuthResponse, AppError> {
        let refresh_token = self.token_service.generate_refresh_token_secret();
        let (session, _refresh_record) =
            self.session_service
                .create_session(&user, provider_name, refresh_token.clone())?;
        let (access_token, expires_in) = self.token_service.issue_access_token(&session)?;
        let tokens = self
            .session_service
            .token_pair(access_token, refresh_token, expires_in);
        Ok(AuthResponse { user, tokens })
    }
}
