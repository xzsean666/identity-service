use std::sync::Arc;

use crate::{
    application::{
        auth::AuthService,
        error::AppError,
        identity_binding::{IdentityBindingService, IdentityRepository},
        provider_registry::ProviderRegistry,
        session::{SessionRepository, SessionService},
        token::TokenService,
    },
    config::{AppConfig, PersistenceBackend},
    infrastructure::{
        memory::{
            InMemoryIdentityRepository, InMemoryLocalCredentialRepository,
            InMemorySessionRepository, InMemoryState,
        },
        postgres::{
            PostgresIdentityRepository, PostgresLocalCredentialRepository,
            PostgresSessionRepository, PostgresState,
        },
    },
    providers::{
        IdentityProviderAdapter,
        local_password::{LocalCredentialRepository, LocalPasswordProvider},
        supabase::SupabaseProvider,
    },
    security::RefreshTokenHasher,
};

pub async fn build_auth_service(config: AppConfig) -> Result<Arc<AuthService>, AppError> {
    let (local_credential_repository, identity_repository, session_repository): (
        Arc<dyn LocalCredentialRepository>,
        Arc<dyn IdentityRepository>,
        Arc<dyn SessionRepository>,
    ) = match config.persistence.backend {
        PersistenceBackend::Memory => {
            let state = InMemoryState::shared();
            (
                Arc::new(InMemoryLocalCredentialRepository::new(state.clone())),
                Arc::new(InMemoryIdentityRepository::new(state.clone())),
                Arc::new(InMemorySessionRepository::new(state)),
            )
        }
        PersistenceBackend::Postgres => {
            let database_url =
                config.persistence.database_url.as_deref().ok_or_else(|| {
                    AppError::Internal("postgres database url is missing".to_owned())
                })?;
            let state = PostgresState::connect(database_url)
                .await
                .map_err(|error| AppError::Internal(error.to_string()))?;
            state
                .health_check()
                .await
                .map_err(|error| AppError::Internal(error.to_string()))?;
            (
                Arc::new(PostgresLocalCredentialRepository::new(state.pool.clone())),
                Arc::new(PostgresIdentityRepository::new(state.pool.clone())),
                Arc::new(PostgresSessionRepository::new(state.pool.clone())),
            )
        }
    };

    let local_password_provider = Arc::new(LocalPasswordProvider::new(
        local_credential_repository,
        config.identity_providers.local_password.enabled,
    ));
    let supabase_provider = Arc::new(SupabaseProvider::new(
        config.identity_providers.supabase.clone(),
    ));
    let provider_registry = ProviderRegistry::new(vec![
        local_password_provider.clone() as Arc<dyn IdentityProviderAdapter>,
        supabase_provider as Arc<dyn IdentityProviderAdapter>,
    ]);
    let identity_binding = IdentityBindingService::new(identity_repository);
    let token_service = TokenService::new(config.tokens.clone())?;
    let session_service = SessionService::new(
        session_repository,
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
