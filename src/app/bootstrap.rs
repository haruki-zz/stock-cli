use crate::app::controller::AppController;
use crate::config::Config;
use crate::error::Result;

/// Entry point used by `main` to bootstrap the controller stack.
pub async fn run() -> Result<()> {
    let config = Config::builtin();
    let controller = AppController::new(config)?;
    controller.run().await
}
