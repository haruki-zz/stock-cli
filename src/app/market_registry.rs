use std::sync::Arc;

use tokio::sync::watch;

use crate::config::{registry::ConfigRegistry, RegionConfig, RegionDescriptor};
use crate::error::{AppError, Result};

/// Lightweight summary used for market pickers.
#[derive(Clone, Debug)]
pub struct MarketSummary {
    pub code: String,
    pub name: String,
}

/// Facade over `ConfigRegistry` that exposes market descriptors to the application/UI layer.
pub struct MarketRegistry {
    registry: Arc<ConfigRegistry>,
}

impl MarketRegistry {
    pub fn new(registry: Arc<ConfigRegistry>) -> Self {
        Self { registry }
    }

    pub fn available_regions(&self) -> Vec<MarketSummary> {
        self.registry
            .snapshot()
            .iter()
            .map(|descriptor| MarketSummary {
                code: descriptor.code.clone(),
                name: descriptor.name.clone(),
            })
            .collect()
    }

    pub fn region_descriptor(&self, code: &str) -> Option<RegionDescriptor> {
        self.registry.get(code)
    }

    pub fn region_config(&self, code: &str) -> Option<RegionConfig> {
        self.region_descriptor(code)
            .map(|descriptor| RegionConfig::from(&descriptor))
    }

    #[allow(dead_code)]
    pub fn subscribe(&self) -> watch::Receiver<Arc<Vec<RegionDescriptor>>> {
        self.registry.subscribe()
    }

    #[allow(dead_code)]
    pub fn refresh(&self) -> Result<()> {
        self.registry.refresh()
    }

    #[allow(dead_code)]
    pub fn start_watching(self: &Arc<Self>) -> Result<()> {
        Arc::clone(&self.registry).start_watching()
    }

    pub fn ensure_region(&self, code: &str) -> Result<RegionConfig> {
        self.region_config(code).ok_or_else(|| {
            AppError::message(format!(
                "Region `{code}` not found. Trigger a reload if new markets were added."
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn exposes_region_summaries() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let registry = Arc::new(ConfigRegistry::new(root).expect("registry"));
        let markets = MarketRegistry::new(Arc::clone(&registry));

        let summaries = markets.available_regions();
        assert!(!summaries.is_empty());
        assert!(summaries.iter().any(|summary| summary.code == "CN"));

        let config = markets.ensure_region("cn").expect("cn config");
        assert_eq!(config.code, "CN");
    }
}
