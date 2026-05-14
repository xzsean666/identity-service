use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;

use crate::application::error::AppError;

#[async_trait]
pub trait ReadinessDependency: Send + Sync {
    fn name(&self) -> &'static str;

    async fn check(&self) -> Result<(), AppError>;
}

#[derive(Clone)]
pub struct ReadinessService {
    persistence: Arc<dyn ReadinessDependency>,
}

#[derive(Serialize)]
pub struct ReadinessReport {
    pub status: &'static str,
    pub persistence: DependencyReadiness,
}

#[derive(Serialize)]
pub struct DependencyReadiness {
    pub name: &'static str,
    pub status: &'static str,
}

impl ReadinessService {
    pub fn new(persistence: Arc<dyn ReadinessDependency>) -> Self {
        Self { persistence }
    }

    pub async fn check(&self) -> Result<ReadinessReport, AppError> {
        self.persistence.check().await?;

        Ok(ReadinessReport {
            status: "ready",
            persistence: DependencyReadiness {
                name: self.persistence.name(),
                status: "ready",
            },
        })
    }
}
