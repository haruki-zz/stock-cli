mod config;
mod fetcher;
mod database;
mod menu;
mod app;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::run().await
}
