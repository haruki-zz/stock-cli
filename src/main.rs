mod app;
mod config;
mod database;
mod fetcher;
mod ui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::run().await
}
