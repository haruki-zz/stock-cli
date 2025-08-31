mod cli;
mod config;
mod stock;

use anyhow::{Context, Result};
use chrono::Local;
use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use cli::{Cli, Commands};
use config::Config;
use stock::{AsyncStockFetcher, StockDatabase};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Interactive => {
            run_interactive_mode(&cli).await?;
        }
        Commands::Update { ref output_dir } => {
            update_stock_data(&cli, output_dir).await?;
        }
        Commands::Show { ref codes, ref from_file } => {
            show_stocks(&cli, codes, from_file.as_deref()).await?;
        }
        Commands::Filter { ref from_file } => {
            filter_stocks(&cli, from_file.as_deref()).await?;
        }
        Commands::Load { ref file } => {
            load_and_show(file).await?;
        }
    }

    Ok(())
}

async fn run_interactive_mode(cli: &Cli) -> Result<()> {
    cli::show_banner();
    
    // Load configuration
    let config = Config::load(&cli.config)
        .context("Failed to load configuration")?;
    
    let region_config = config.get_region_config(&cli.region)
        .context("Region not found in config")?
        .clone();
    
    let info_indices = config.get_valid_info_indices(&cli.region)
        .context("No valid info indices found")?;
    
    let thresholds = config.get_valid_thresholds(&cli.region)
        .unwrap_or_default();

    // Load stock codes
    let stock_codes = load_stock_codes(&cli.stock_codes)?;
    
    // Create raw data directory
    let raw_data_dir = "raw_data";
    if !Path::new(raw_data_dir).exists() {
        fs::create_dir_all(raw_data_dir)
            .context("Failed to create raw_data directory")?;
    }

    // Check for existing data
    let mut database = load_or_fetch_data(
        raw_data_dir,
        &stock_codes,
        region_config.clone(),
        info_indices.clone(),
    ).await?;

    // Interactive loop
    loop {
        print!("Waiting for command: ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)
            .context("Failed to read user input")?;
        
        let input = input.trim().to_lowercase();
        let parts: Vec<&str> = input.split_whitespace().collect();
        
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "exit" => {
                println!("Exiting...");
                break;
            }
            "show" => {
                if parts.len() > 1 {
                    let codes: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();
                    database.show_stock_info(&codes);
                } else {
                    println!("Usage: show <stock_code1> [stock_code2] ...");
                }
            }
            "update" => {
                println!("Fetching new data...");
                match fetch_new_data(raw_data_dir, &stock_codes, 
                                   region_config.clone(), info_indices.clone()).await {
                    Ok(new_data) => {
                        database.update(new_data);
                        println!("Update stock information successfully.");
                    }
                    Err(e) => {
                        println!("Failed to update stock information: {}", e);
                    }
                }
            }
            "filter" => {
                println!("Filtering stock with default thresholds...");
                let filtered_codes = database.filter_stocks(&thresholds);
                println!("Filtering results:");
                database.show_stock_info(&filtered_codes);
            }
            "load" => {
                if parts.len() > 1 {
                    match StockDatabase::load_from_csv(parts[1]) {
                        Ok(loaded_db) => {
                            database = loaded_db;
                            println!("Data loaded from {}", parts[1]);
                        }
                        Err(e) => {
                            println!("Failed to load data from {}: {}", parts[1], e);
                        }
                    }
                } else {
                    println!("Usage: load <filename>");
                }
            }
            _ => {
                println!("Unknown command. Available commands: show, update, filter, load, exit");
            }
        }
    }

    Ok(())
}

async fn update_stock_data(cli: &Cli, output_dir: &str) -> Result<()> {
    let config = Config::load(&cli.config)?;
    let region_config = config.get_region_config(&cli.region)
        .context("Region not found in config")?
        .clone();
    let info_indices = config.get_valid_info_indices(&cli.region)
        .context("No valid info indices found")?;
    let stock_codes = load_stock_codes(&cli.stock_codes)?;

    if !Path::new(output_dir).exists() {
        fs::create_dir_all(output_dir)?;
    }

    let data = fetch_new_data(output_dir, &stock_codes, region_config, info_indices).await?;
    let database = StockDatabase::new(data);
    
    let timestamp = Local::now().format("%Y_%m_%d_%H_%M");
    let filename = format!("{}/{}_raw.csv", output_dir, timestamp);
    database.save_to_csv(&filename)?;
    
    println!("Data saved to {}", filename);
    Ok(())
}

async fn show_stocks(cli: &Cli, codes: &[String], from_file: Option<&str>) -> Result<()> {
    let database = if let Some(file_path) = from_file {
        StockDatabase::load_from_csv(file_path)
            .context("Failed to load data from file")?
    } else {
        // Need to fetch fresh data
        let config = Config::load(&cli.config)?;
        let region_config = config.get_region_config(&cli.region)
            .context("Region not found in config")?
            .clone();
        let info_indices = config.get_valid_info_indices(&cli.region)
            .context("No valid info indices found")?;
        let stock_codes = load_stock_codes(&cli.stock_codes)?;
        
        let fetcher = AsyncStockFetcher::new(stock_codes, region_config, info_indices);
        let data = fetcher.fetch_data().await?;
        StockDatabase::new(data)
    };

    database.show_stock_info(codes);
    Ok(())
}

async fn filter_stocks(cli: &Cli, from_file: Option<&str>) -> Result<()> {
    let config = Config::load(&cli.config)?;
    let thresholds = config.get_valid_thresholds(&cli.region)
        .unwrap_or_default();

    let database = if let Some(file_path) = from_file {
        StockDatabase::load_from_csv(file_path)
            .context("Failed to load data from file")?
    } else {
        // Need to fetch fresh data
        let region_config = config.get_region_config(&cli.region)
            .context("Region not found in config")?
            .clone();
        let info_indices = config.get_valid_info_indices(&cli.region)
            .context("No valid info indices found")?;
        let stock_codes = load_stock_codes(&cli.stock_codes)?;
        
        let fetcher = AsyncStockFetcher::new(stock_codes, region_config, info_indices);
        let data = fetcher.fetch_data().await?;
        StockDatabase::new(data)
    };

    let filtered_codes = database.filter_stocks(&thresholds);
    println!("Filtering results:");
    database.show_stock_info(&filtered_codes);
    Ok(())
}

async fn load_and_show(file_path: &str) -> Result<()> {
    let database = StockDatabase::load_from_csv(file_path)
        .context("Failed to load data from file")?;
    
    println!("Loaded {} stock records from {}", database.data.len(), file_path);
    
    // Show first few records as sample
    let sample_codes: Vec<String> = database.data
        .iter()
        .take(5)
        .map(|stock| stock.stock_code.clone())
        .collect();
    
    if !sample_codes.is_empty() {
        println!("Sample data:");
        database.show_stock_info(&sample_codes);
    }
    
    Ok(())
}

async fn load_or_fetch_data(
    raw_data_dir: &str,
    stock_codes: &[String],
    region_config: config::RegionConfig,
    info_indices: HashMap<String, config::InfoIndex>,
) -> Result<StockDatabase> {
    let data_files: Vec<_> = fs::read_dir(raw_data_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "csv")
                .unwrap_or(false)
        })
        .collect();

    if data_files.is_empty() {
        println!("No previous data detected. Start fetching new data by default...");
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        let data = fetch_new_data(raw_data_dir, stock_codes, region_config, info_indices).await?;
        Ok(StockDatabase::new(data))
    } else {
        let latest_file = data_files
            .into_iter()
            .max_by_key(|entry| entry.metadata().unwrap().modified().unwrap())
            .unwrap();
        
        let latest_file_name = latest_file.file_name();
        let latest_file_name_str = latest_file_name.to_string_lossy();
        
        print!("Previous data detected. Load data from latest file {}? (y/n): ", latest_file_name_str);
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        
        if input == "y" {
            println!("Data loaded from {}.", latest_file_name_str);
            let file_path = latest_file.path();
            StockDatabase::load_from_csv(file_path.to_str().unwrap())
                .context("Failed to load existing data")
        } else {
            println!("Start to fetch new data...");
            let data = fetch_new_data(raw_data_dir, stock_codes, region_config, info_indices).await?;
            Ok(StockDatabase::new(data))
        }
    }
}

async fn fetch_new_data(
    raw_data_dir: &str,
    stock_codes: &[String],
    region_config: config::RegionConfig,
    info_indices: HashMap<String, config::InfoIndex>,
) -> Result<Vec<stock::StockData>> {
    let timestamp = Local::now().format("%Y_%m_%d_%H_%M");
    println!("Start to fetch real-time data at time {} ...", timestamp);
    
    let fetcher = AsyncStockFetcher::new(
        stock_codes.to_vec(),
        region_config,
        info_indices,
    );
    
    let data = fetcher.fetch_data().await
        .context("Failed to fetch stock data")?;
    
    println!("Fetching complete. Start to save data...");
    
    let database = StockDatabase::new(data.clone());
    let filename = format!("{}/{}_raw.csv", raw_data_dir, timestamp);
    database.save_to_csv(&filename)
        .context("Failed to save data to CSV")?;
    
    println!("Finished. Real-time data information updated successfully...");
    
    Ok(data)
}

fn get_default_stock_codes() -> Vec<String> {
    vec![
        "sh000001", "sz399001", "sh600000", "sz000001", "sh600036",
        "sz000002", "sh600519", "sh601318", "sz300059", "sh688981"
    ].into_iter().map(String::from).collect()
}

fn load_stock_codes(file_path: &str) -> Result<Vec<String>> {
    if !Path::new(file_path).exists() {
        let sample_codes = get_default_stock_codes();
        
        // Try to create the file, but don't fail if we can't (e.g., read-only filesystem)
        if let Ok(mut writer) = csv::Writer::from_path(file_path) {
            for code in &sample_codes {
                let _ = writer.write_record(&[code]);
            }
            let _ = writer.flush();
            println!("Created sample stock codes file: {}", file_path);
        } else {
            println!("Using default stock codes (couldn't create file: {})", file_path);
        }
        
        return Ok(sample_codes);
    }

    let mut reader = csv::Reader::from_path(file_path)
        .context("Failed to open stock codes file")?;
    
    let mut codes = Vec::new();
    for result in reader.records() {
        let record = result.context("Failed to read CSV record")?;
        if let Some(code) = record.get(0) {
            codes.push(code.to_string());
        }
    }
    
    if codes.is_empty() {
        println!("Empty stock codes file, using defaults");
        return Ok(get_default_stock_codes());
    }
    
    Ok(codes)
}
