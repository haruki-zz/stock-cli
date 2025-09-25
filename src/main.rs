mod app;
mod config;
mod core;
mod database;
mod ui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Delegate to the async application harness that wires configuration, fetching, and the UI.
    app::run().await
}
