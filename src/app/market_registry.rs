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

    pub fn subscribe(&self) -> watch::Receiver<Arc<Vec<RegionDescriptor>>> {
        self.registry.subscribe()
    }

    pub fn refresh(&self) -> Result<()> {
        self.registry.refresh()
    }

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
