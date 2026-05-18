use std::sync::Arc;

use crate::{
    application::{
        auth::{AuthService, LocalRegistrationRepository},
        error::AppError,
        identity_binding::{IdentityBindingService, IdentityRepository},
        password_change::{PasswordChangeRepository, PasswordChangeService},
        provider_registry::ProviderRegistry,
        readiness::{ReadinessDependency, ReadinessService},
        session::{SessionRepository, SessionService},
        token::TokenService,
    },
    config::{AppConfig, HttpConfig, PersistenceBackend},
    infrastructure::{
        memory::{
            InMemoryIdentityRepository, InMemoryLocalCredentialRepository,
            InMemoryPasswordChangeRepository, InMemoryReadinessCheck, InMemorySessionRepository,
            InMemoryState,
        },
        postgres::{
            PostgresIdentityRepository, PostgresLocalCredentialRepository,
            PostgresPasswordChangeRepository, PostgresReadinessCheck, PostgresSessionRepository,
            PostgresState,
        },
        sqlite::{
            SqliteIdentityRepository, SqliteLocalCredentialRepository,
            SqlitePasswordChangeRepository, SqliteReadinessCheck, SqliteSessionRepository,
            SqliteState,
        },
    },
    providers::{
        IdentityProviderAdapter,
        local_password::{LocalCredentialRepository, LocalPasswordProvider},
        supabase::SupabaseProvider,
    },
    security::RefreshTokenHasher,
};

#[derive(Clone)]
pub struct ApplicationServices {
    pub auth_service: Arc<AuthService>,
    pub readiness_service: ReadinessService,
    pub http: HttpConfig,
}

struct PersistenceServices {
    local_credential_repository: Arc<dyn LocalCredentialRepository>,
    identity_repository: Arc<dyn IdentityRepository>,
    local_registration_repository: Arc<dyn LocalRegistrationRepository>,
    session_repository: Arc<dyn SessionRepository>,
    password_change_repository: Arc<dyn PasswordChangeRepository>,
    readiness_dependency: Arc<dyn ReadinessDependency>,
}

pub async fn build_auth_service(config: AppConfig) -> Result<Arc<AuthService>, AppError> {
    build_application_services(config)
        .await
        .map(|services| services.auth_service.clone())
}

pub async fn build_application_services(
    config: AppConfig,
) -> Result<Arc<ApplicationServices>, AppError> {
    let http = config.http.clone();
    let persistence_services = match config.persistence.backend {
        PersistenceBackend::Memory => {
            let state = InMemoryState::shared();
            let identity_repository = Arc::new(InMemoryIdentityRepository::new(state.clone()));
            PersistenceServices {
                local_credential_repository: Arc::new(InMemoryLocalCredentialRepository::new(
                    state.clone(),
                )),
                identity_repository: identity_repository.clone(),
                local_registration_repository: identity_repository,
                session_repository: Arc::new(InMemorySessionRepository::new(state.clone())),
                password_change_repository: Arc::new(InMemoryPasswordChangeRepository::new(state)),
                readiness_dependency: Arc::new(InMemoryReadinessCheck),
            }
        }
        PersistenceBackend::Sqlite => {
            let database_url =
                config.persistence.database_url.as_deref().ok_or_else(|| {
                    AppError::Internal("sqlite database url is missing".to_owned())
                })?;
            let state = SqliteState::connect(database_url)
                .await
                .map_err(|error| AppError::Internal(error.to_string()))?;
            state
                .health_check()
                .await
                .map_err(|error| AppError::Internal(error.to_string()))?;
            let identity_repository = Arc::new(SqliteIdentityRepository::new(state.pool.clone()));
            PersistenceServices {
                local_credential_repository: Arc::new(SqliteLocalCredentialRepository::new(
                    state.pool.clone(),
                )),
                identity_repository: identity_repository.clone(),
                local_registration_repository: identity_repository,
                session_repository: Arc::new(SqliteSessionRepository::new(state.pool.clone())),
                password_change_repository: Arc::new(SqlitePasswordChangeRepository::new(
                    state.pool.clone(),
                )),
                readiness_dependency: Arc::new(SqliteReadinessCheck::new(state.pool.clone())),
            }
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
            let identity_repository = Arc::new(PostgresIdentityRepository::new(state.pool.clone()));
            PersistenceServices {
                local_credential_repository: Arc::new(PostgresLocalCredentialRepository::new(
                    state.pool.clone(),
                )),
                identity_repository: identity_repository.clone(),
                local_registration_repository: identity_repository,
                session_repository: Arc::new(PostgresSessionRepository::new(state.pool.clone())),
                password_change_repository: Arc::new(PostgresPasswordChangeRepository::new(
                    state.pool.clone(),
                )),
                readiness_dependency: Arc::new(PostgresReadinessCheck::new(state.pool.clone())),
            }
        }
    };

    let local_password_provider = Arc::new(LocalPasswordProvider::new(
        persistence_services.local_credential_repository,
        config.identity_providers.local_password.enabled,
    ));
    let supabase_provider = Arc::new(SupabaseProvider::new(
        config.identity_providers.supabase.clone(),
    ));
    let provider_registry = ProviderRegistry::new(vec![
        local_password_provider.clone() as Arc<dyn IdentityProviderAdapter>,
        supabase_provider as Arc<dyn IdentityProviderAdapter>,
    ]);
    let identity_binding = IdentityBindingService::new(persistence_services.identity_repository);
    let token_service = TokenService::new(config.tokens.clone())?;
    let session_service = SessionService::new(
        persistence_services.session_repository,
        config.sessions.clone(),
        config.client,
        RefreshTokenHasher::new(config.security.refresh_token_hmac_secret.clone()),
    );
    let password_change_service = PasswordChangeService::new(
        persistence_services.password_change_repository,
        config.sessions,
        RefreshTokenHasher::new(config.security.refresh_token_hmac_secret),
    );

    let auth_service = Arc::new(AuthService::new(
        provider_registry,
        identity_binding,
        persistence_services.local_registration_repository,
        session_service,
        password_change_service,
        token_service,
        local_password_provider,
        config.identity_providers.supabase,
    ));
    let readiness_service = ReadinessService::new(persistence_services.readiness_dependency);

    Ok(Arc::new(ApplicationServices {
        auth_service,
        readiness_service,
        http,
    }))
}
