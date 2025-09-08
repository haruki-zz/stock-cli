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
    
    let mut thresholds = config.get_valid_thresholds(region)
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
            MenuAction::SetThresholds => {
                if let Err(e) = set_thresholds_interactively(&mut thresholds) {
                    println!("Failed to set thresholds: {}", e);
                }
                wait_for_key();
            }
            MenuAction::Filter => {
                println!("Filtering stocks with thresholds (valid only):");
                display_thresholds(&thresholds);
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

fn display_thresholds(thresholds: &std::collections::HashMap<String, config::Threshold>) {
    use unicode_width::UnicodeWidthStr;
    if thresholds.is_empty() {
        println!("  (no thresholds)");
        return;
    }

    let mut items: Vec<_> = thresholds
        .iter()
        .filter(|(_, t)| t.valid)
        .map(|(k, t)| (k.as_str(), t.lower, t.upper))
        .collect();
    items.sort_by(|a, b| a.0.cmp(b.0));

    let name_width = items
        .iter()
        .map(|(k, _, _)| UnicodeWidthStr::width(*k))
        .max()
        .unwrap_or(4)
        .max("Metric".len());

    let header = format!("{:<name_w$} | Lower  | Upper  ", "Metric", name_w = name_width);
    println!("{}", header);
    println!("{}", "-".repeat(header.len()));
    for (k, lo, up) in items {
        println!("{:<name_w$} | {:>6.2} | {:>6.2}", k, lo, up, name_w = name_width);
    }
    println!();
}

fn read_line_trimmed() -> anyhow::Result<String> {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn prompt_f64(label: &str, default: f64) -> anyhow::Result<f64> {
    loop {
        print!("{} (default {:.4}): ", label, default);
        std::io::Write::flush(&mut std::io::stdout())?;
        let s = read_line_trimmed()?;
        if s.is_empty() {
            return Ok(default);
        }
        match s.parse::<f64>() {
            Ok(v) => return Ok(v),
            Err(_) => {
                println!("Invalid number, please try again.");
            }
        }
    }
}

fn set_thresholds_interactively(
    thresholds: &mut std::collections::HashMap<String, config::Threshold>,
) -> anyhow::Result<()> {
    use crossterm::{
        event::{self, Event, KeyCode},
        style::{Attribute, SetAttribute},
        terminal::{self, ClearType},
        QueueableCommand,
    };
    use std::io::{stdout, Write};

    let mut stdout = stdout();
    terminal::enable_raw_mode()?;

    let mut selected: usize = 0;

    let mut keys: Vec<String> = thresholds.keys().cloned().collect();
    keys.sort();

    // Helper to (re)draw the selection list
    let mut redraw = |keys: &Vec<String>, selected: usize, thresholds: &std::collections::HashMap<String, config::Threshold>| -> anyhow::Result<()> {
        stdout.queue(terminal::Clear(ClearType::All))?;
        stdout.write_all(b"Set Thresholds (Use Up/Down, Enter to edit, Esc to exit)\r\n\r\n")?;

        for (i, k) in keys.iter().enumerate() {
            let thr = thresholds.get(k).unwrap();
            let is_sel = i == selected;
            // Arrow (never reversed)
            stdout.write_all(if is_sel { "► ".as_bytes() } else { "  ".as_bytes() })?;
            // Content (reversed when selected)
            if is_sel { stdout.queue(SetAttribute(Attribute::Reverse))?; }
            stdout.write_all(
                format!("{:<12} : [{:.2}, {:.2}]", k, thr.lower, thr.upper).as_bytes(),
            )?;
            if is_sel { stdout.queue(SetAttribute(Attribute::Reset))?; }
            stdout.write_all("\r\n".as_bytes())?;
        }
        // Extra options
        let is_add = selected == keys.len();
        // Arrow (never reversed)
        stdout.write_all(if is_add { "► ".as_bytes() } else { "  ".as_bytes() })?;
        // Content (reversed when selected)
        if is_add { stdout.queue(SetAttribute(Attribute::Reverse))?; }
        stdout.write_all("Add new metric".as_bytes())?;
        if is_add { stdout.queue(SetAttribute(Attribute::Reset))?; }
        stdout.write_all("\r\n".as_bytes())?;

        let is_done = selected == keys.len() + 1;
        // Arrow (never reversed)
        stdout.write_all(if is_done { "► ".as_bytes() } else { "  ".as_bytes() })?;
        // Content (reversed when selected)
        if is_done { stdout.queue(SetAttribute(Attribute::Reverse))?; }
        stdout.write_all("Done".as_bytes())?;
        if is_done { stdout.queue(SetAttribute(Attribute::Reset))?; }
        stdout.write_all("\r\n".as_bytes())?;

        stdout.flush()?;
        Ok(())
    };

    redraw(&keys, selected, thresholds)?;

    loop {
        match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Up => {
                    let total = keys.len() + 2; // include Add and Done
                    selected = (selected + total - 1) % total;
                    redraw(&keys, selected, thresholds)?;
                }
                KeyCode::Down => {
                    let total = keys.len() + 2; // include Add and Done
                    selected = (selected + 1) % total;
                    redraw(&keys, selected, thresholds)?;
                }
                KeyCode::Enter => {
                    // Temporarily disable raw mode for line input
                    terminal::disable_raw_mode()?;
                    if selected < keys.len() {
                        let name = keys[selected].clone();
                        if let Some(existing) = thresholds.get(&name).cloned() {
                            println!("\nEditing '{}' (current [{:.2}, {:.2}])", name, existing.lower, existing.upper);
                            let lower = prompt_f64("  Lower bound", existing.lower)?;
                            let upper = prompt_f64("  Upper bound", existing.upper)?;
                            let (lo, up) = if lower <= upper { (lower, upper) } else { (upper, lower) };
                            thresholds.insert(
                                name,
                                config::Threshold { lower: lo, upper: up, valid: true },
                            );
                        }
                    } else if selected == keys.len() {
                        println!("\nEnter new metric name: ");
                        let name = read_line_trimmed()?;
                        if !name.is_empty() {
                            let lower = prompt_f64("  Lower bound", 0.0)?;
                            let upper = prompt_f64("  Upper bound", lower)?;
                            let (lo, up) = if lower <= upper { (lower, upper) } else { (upper, lower) };
                            thresholds.insert(
                                name.clone(),
                                config::Threshold { lower: lo, upper: up, valid: true },
                            );
                            if !keys.contains(&name) {
                                keys.push(name);
                                keys.sort();
                            }
                        } else {
                            println!("Metric name cannot be empty. Press Enter to continue...");
                            let _ = read_line_trimmed();
                        }
                    } else {
                        // Done
                        break;
                    }
                    // Re-enable raw mode and redraw
                    terminal::enable_raw_mode()?;
                    selected = 0.min(keys.len());
                    redraw(&keys, selected, thresholds)?;
                }
                KeyCode::Esc => break,
                _ => {}
            },
            _ => {}
        }
    }

    terminal::disable_raw_mode()?;
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
