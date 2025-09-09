mod config;
mod fetcher;
mod database;
mod menu;
mod app;
mod ui;
mod threshold_menu;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::run().await
}
