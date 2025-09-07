mod config;
mod fetcher;
mod database;
mod menu;

use anyhow::{Context, Result};
use chrono::Local;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use config::Config;
use fetcher::{AsyncStockFetcher, StockData};
use database::StockDatabase;
use menu::{Menu, MenuAction};

#[tokio::main]
async fn main() -> Result<()> {
    run_interactive_mode().await
}

async fn run_interactive_mode() -> Result<()> {
    // Default configuration paths
    let config_path = "config.json";
    let stock_codes_path = "stock_code.csv";
    let region = "CN";
    
    // Load configuration
    let config = Config::load(config_path)
        .context("Failed to load configuration")?;
    
    let region_config = config.get_region_config(region)
        .context("Region not found in config")?
        .clone();
    
    let info_indices = config.get_valid_info_indices(region)
        .context("No valid info indices found")?;
    
    let thresholds = config.get_valid_thresholds(region)
        .unwrap_or_default();

    // Load stock codes
    let stock_codes = load_stock_codes(stock_codes_path)?;
    
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

    // Main interactive loop
    loop {
        let mut menu = Menu::new();
        let action = menu.navigate()?;

        match action {
            MenuAction::Update => {
                println!("Fetching new data...");
                match fetch_new_data(raw_data_dir, &stock_codes, 
                                   region_config.clone(), info_indices.clone()).await {
                    Ok(new_data) => {
                        database.update(new_data);
                        println!("Stock information updated successfully.");
                    }
                    Err(e) => {
                        println!("Failed to update stock information: {}", e);
                    }
                }
                wait_for_key();
            }
            MenuAction::Show => {
                let codes = get_stock_codes_input()?;
                if !codes.is_empty() {
                    database.show_stock_info(&codes);
                } else {
                    println!("No stock codes entered.");
                }
                wait_for_key();
            }
            MenuAction::Filter => {
                println!("Filtering stocks with default thresholds...");
                let filtered_codes = database.filter_stocks(&thresholds);
                if filtered_codes.is_empty() {
                    println!("No stocks match the filtering criteria.");
                } else {
                    println!("Filtering results:");
                    database.show_stock_info(&filtered_codes);
                }
                wait_for_key();
            }
            MenuAction::Load => {
                let filename = get_filename_input()?;
                if !filename.is_empty() {
                    match StockDatabase::load_from_csv(&filename) {
                        Ok(loaded_db) => {
                            database = loaded_db;
                            println!("Data loaded from {}", filename);
                        }
                        Err(e) => {
                            println!("Failed to load data from {}: {}", filename, e);
                        }
                    }
                } else {
                    println!("No filename entered.");
                }
                wait_for_key();
            }
            MenuAction::Exit => {
                println!("Exiting...");
                break;
            }
        }
    }

    Ok(())
}

fn get_stock_codes_input() -> Result<Vec<String>> {
    print!("Enter stock codes (separated by spaces): ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let codes: Vec<String> = input
        .trim()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    
    Ok(codes)
}

fn get_filename_input() -> Result<String> {
    print!("Enter filename: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    Ok(input.trim().to_string())
}

fn wait_for_key() {
    println!("\nPress Enter to continue...");
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
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
        println!("No previous data detected. Fetching new data...");
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
            println!("Fetching new data...");
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
) -> Result<Vec<StockData>> {
    let timestamp = Local::now().format("%Y_%m_%d_%H_%M");
    println!("Fetching real-time data at {} ...", timestamp);
    
    let fetcher = AsyncStockFetcher::new(
        stock_codes.to_vec(),
        region_config,
        info_indices,
    );
    
    let data = fetcher.fetch_data().await
        .context("Failed to fetch stock data")?;
    
    println!("Fetching complete. Saving data...");
    
    let database = StockDatabase::new(data.clone());
    let filename = format!("{}/{}_raw.csv", raw_data_dir, timestamp);
    database.save_to_csv(&filename)
        .context("Failed to save data to CSV")?;
    
    println!("Data saved successfully to {}", filename);
    
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
