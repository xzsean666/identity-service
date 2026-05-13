use std::sync::Arc;

use crate::{
    application::{
        auth::AuthService, error::AppError, identity_binding::IdentityBindingService,
        provider_registry::ProviderRegistry, session::SessionService, token::TokenService,
    },
    config::AppConfig,
    infrastructure::memory::{
        InMemoryIdentityRepository, InMemoryLocalCredentialRepository, InMemorySessionRepository,
        InMemoryState,
    },
    providers::{
        IdentityProviderAdapter, local_password::LocalPasswordProvider, supabase::SupabaseProvider,
    },
    security::RefreshTokenHasher,
};

pub fn build_auth_service(config: AppConfig) -> Result<Arc<AuthService>, AppError> {
    let state = InMemoryState::shared();
    let local_password_provider = Arc::new(LocalPasswordProvider::new(
        Arc::new(InMemoryLocalCredentialRepository::new(state.clone())),
        config.identity_providers.local_password.enabled,
    ));
    let supabase_provider = Arc::new(SupabaseProvider::new(
        config.identity_providers.supabase.clone(),
    ));
    let provider_registry = ProviderRegistry::new(vec![
        local_password_provider.clone() as Arc<dyn IdentityProviderAdapter>,
        supabase_provider as Arc<dyn IdentityProviderAdapter>,
    ]);
    let identity_binding =
        IdentityBindingService::new(Arc::new(InMemoryIdentityRepository::new(state.clone())));
    let token_service = TokenService::new(config.tokens.clone())?;
    let session_service = SessionService::new(
        Arc::new(InMemorySessionRepository::new(state)),
        config.sessions,
        config.client,
        RefreshTokenHasher::new(config.security.refresh_token_hmac_secret),
    );

    Ok(Arc::new(AuthService::new(
        provider_registry,
        identity_binding,
        session_service,
        token_service,
        local_password_provider,
        config.identity_providers.supabase,
    )))
}
