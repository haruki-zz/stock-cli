use std::env;

use env_logger::Env;
use log::{info, LevelFilter};
use stock_cli::app;
use stock_cli::error::{AppError, Result};

#[derive(Debug, Clone)]
struct CliOptions {
    log_level: Option<LevelFilter>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = parse_cli_options()?;
    init_logging(cli.log_level)?;
    info!("Starting stock-cli");
    app::run().await
}

fn parse_cli_options() -> Result<CliOptions> {
    let mut args = env::args().skip(1);
    let mut log_level = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--log-level" => {
                let value = args.next().ok_or_else(|| {
                    AppError::message(
                        "--log-level requires a value (error, warn, info, debug, trace)",
                    )
                })?;
                let level = value.parse::<LevelFilter>().map_err(|_| {
                    AppError::message(format!(
                        "Invalid log level '{}'. Expected error, warn, info, debug, or trace.",
                        value
                    ))
                })?;
                log_level = Some(level);
            }
            "--quiet" | "-q" => {
                log_level = Some(LevelFilter::Warn);
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other if other.starts_with('-') => {
                return Err(AppError::message(format!("Unknown option '{}'", other)));
            }
            other => {
                return Err(AppError::message(format!(
                    "Unexpected argument '{}'",
                    other
                )));
            }
        }
    }

    Ok(CliOptions { log_level })
}

fn init_logging(level: Option<LevelFilter>) -> Result<()> {
    let mut builder = env_logger::Builder::from_env(Env::default());

    if let Some(level) = level {
        builder.filter_level(level);
    } else if env::var("RUST_LOG").is_err() {
        builder.filter_level(LevelFilter::Info);
    }

    builder
        .format_timestamp_secs()
        .format_target(false)
        .try_init()
        .map_err(|err| AppError::message(format!("Failed to initialize logger: {}", err)))
}

fn print_usage() {
    println!(
        "Stock CLI\n\nUSAGE:\n    stock-cli [OPTIONS]\n\nOPTIONS:\n    --log-level <LEVEL>    Override the default log level (error, warn, info, debug, trace)\n    -q, --quiet            Reduce logging noise (equivalent to --log-level warn)\n    -h, --help             Show this help message\n\nEnvironment variables:\n    RUST_LOG               Standard env_logger filter string."
    );
}
