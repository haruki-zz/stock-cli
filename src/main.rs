mod application;
mod config;
mod error;
mod fetch;
mod records;
mod ui;
mod utils;

use crate::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Delegate to the async application harness that wires configuration, fetching, and the UI.
    application::run().await
}
