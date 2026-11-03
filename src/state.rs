use crate::config::Config;
use crate::providers::ProviderRegistry;
use crate::services::token::JwtEngine;
use axum::extract::FromRef;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_engine: Arc<JwtEngine>,
    pub config: Arc<Config>,
    pub providers: Arc<ProviderRegistry>,
}

impl AppState {
    pub fn new(
        pool: PgPool,
        jwt_engine: JwtEngine,
        config: Config,
        providers: ProviderRegistry,
    ) -> Self {
        Self {
            pool,
            jwt_engine: Arc::new(jwt_engine),
            config: Arc::new(config),
            providers: Arc::new(providers),
        }
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for Arc<JwtEngine> {
    fn from_ref(state: &AppState) -> Self {
        state.jwt_engine.clone()
    }
}

impl FromRef<AppState> for JwtEngine {
    fn from_ref(state: &AppState) -> Self {
        (*state.jwt_engine).clone()
    }
}

impl FromRef<AppState> for Arc<Config> {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

impl FromRef<AppState> for Arc<ProviderRegistry> {
    fn from_ref(state: &AppState) -> Self {
        state.providers.clone()
    }
}
