mod application;
mod config;
mod services;
mod storage;
mod ui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Delegate to the async application harness that wires configuration, fetching, and the UI.
    application::run().await
}
