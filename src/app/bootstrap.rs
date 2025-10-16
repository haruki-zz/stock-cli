use std::sync::Arc;

use crate::app::{controller::AppController, market_registry::MarketRegistry};
use crate::config::registry::ConfigRegistry;
use crate::error::{Context, Result};

/// Entry point used by `main` to bootstrap the controller stack.
pub async fn run() -> Result<()> {
    let root = std::env::current_dir().context("Failed to determine project root")?;
    let config_registry = Arc::new(ConfigRegistry::new(root)?);
    config_registry
        .start_watching()
        .context("Failed to start config watcher")?;

    let market_registry = Arc::new(MarketRegistry::new(Arc::clone(&config_registry)));
    let controller = AppController::new(Arc::clone(&market_registry))?;
    controller.run().await
}
