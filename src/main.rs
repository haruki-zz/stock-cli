mod config;
mod fetcher;
mod database;
mod ui;
mod app;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::run().await
}
