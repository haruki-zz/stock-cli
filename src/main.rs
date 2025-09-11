mod config;
mod fetcher;
mod database;
mod menu;
mod app;
mod ui;
mod threshold_menu;
mod select;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::run().await
}
