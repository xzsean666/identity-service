use std::{collections::HashMap, sync::Arc};

use crate::{
    application::error::AppError,
    providers::{IdentityProviderAdapter, ProviderDescriptor},
};

#[derive(Clone)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn IdentityProviderAdapter>>,
    descriptors: HashMap<String, ProviderDescriptor>,
}

impl ProviderRegistry {
    pub fn new(providers: Vec<Arc<dyn IdentityProviderAdapter>>) -> Self {
        let mut provider_map = HashMap::new();
        let mut descriptor_map = HashMap::new();

        for provider in providers {
            let descriptor = provider.descriptor();
            descriptor_map.insert(descriptor.provider_name.to_owned(), descriptor.clone());
            provider_map.insert(descriptor.provider_name.to_owned(), provider);
        }

        Self {
            providers: provider_map,
            descriptors: descriptor_map,
        }
    }

    pub fn provider(
        &self,
        provider_name: &str,
    ) -> Result<Arc<dyn IdentityProviderAdapter>, AppError> {
        let descriptor = self
            .descriptors
            .get(provider_name)
            .ok_or(AppError::ProviderVerificationFailed)?;

        if !descriptor.enabled {
            return Err(AppError::ProviderDisabled);
        }

        self.providers
            .get(provider_name)
            .cloned()
            .ok_or_else(|| AppError::Internal("enabled provider is missing adapter".to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        infrastructure::memory::{InMemoryLocalCredentialRepository, InMemoryState},
        providers::{IdentityProviderAdapter, local_password::LocalPasswordProvider},
    };

    #[test]
    fn disabled_provider_returns_provider_disabled() {
        let provider: Arc<dyn IdentityProviderAdapter> = Arc::new(LocalPasswordProvider::new(
            Arc::new(InMemoryLocalCredentialRepository::new(
                InMemoryState::shared(),
            )),
            false,
        ));
        let registry = ProviderRegistry::new(vec![provider]);

        assert!(matches!(
            registry.provider("local_password"),
            Err(AppError::ProviderDisabled)
        ));
    }
}
