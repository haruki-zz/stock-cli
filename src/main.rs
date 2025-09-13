mod config;
mod fetcher;
mod database;
mod ui;
mod app;
mod action;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::run().await
}
