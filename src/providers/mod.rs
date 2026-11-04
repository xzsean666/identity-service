use crate::config::Config;
use crate::error::AppError;
use crate::models::ExternalIdentity;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub mod google;
pub mod mock;
pub mod supabase;
pub mod x;

pub use google::GoogleProvider;
pub use mock::MockProvider;
pub use supabase::SupabaseProvider;
pub use x::XProvider;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait ProviderVerifier: Send + Sync {
    fn provider_name(&self) -> &'static str;
    fn verify<'a>(&'a self, token: &'a str) -> BoxFuture<'a, Result<ExternalIdentity, AppError>>;
}

#[derive(Clone)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn ProviderVerifier>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub fn register(&mut self, provider: Arc<dyn ProviderVerifier>) {
        self.providers
            .insert(provider.provider_name().to_lowercase(), provider);
    }

    pub fn from_config(config: &Config) -> Self {
        let mut registry = Self::new();

        if config.enable_mock_provider {
            registry.register(Arc::new(MockProvider::new()));
        }
        registry.register(Arc::new(GoogleProvider::new(
            config.google_client_id.clone(),
            config.env.is_production(),
        )));
        registry.register(Arc::new(XProvider::new(config.x_client_id.clone())));
        registry.register(Arc::new(SupabaseProvider::new(
            config.supabase_url.clone(),
            config.supabase_jwt_secret.clone(),
            config.env.is_production(),
        )));

        registry
    }

    pub fn get(&self, provider: &str) -> Result<Arc<dyn ProviderVerifier>, AppError> {
        self.providers
            .get(&provider.to_lowercase())
            .cloned()
            .ok_or_else(|| {
                AppError::BadRequest(format!("Unsupported authentication provider: '{provider}'"))
            })
    }

    pub async fn verify(
        &self,
        provider: &str,
        token: &str,
    ) -> Result<ExternalIdentity, AppError> {
        let verifier = self.get(provider)?;
        verifier.verify(token).await
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
