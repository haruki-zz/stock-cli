use anyhow::{Context, Result};
use std::fs;
use std::io;
use std::path::Path;

use crate::config::Config;
use crate::database::StockDatabase;
use crate::ui::menu_main::MenuAction;
use crate::ui::ratatui_app::{run_main_menu, run_csv_picker, run_thresholds_editor, run_results_table, run_fetch_progress};
use crate::action::find_latest_csv;
use crossterm::{cursor, terminal::{self, ClearType}, QueueableCommand};

pub async fn run() -> Result<()> {
    let config_path = "config.json";
    let stock_codes_path = "stock_code.csv";
    let region = "CN";

    // Load configuration
    let config = Config::load(config_path).context("Failed to load configuration")?;

    let region_config = config
        .get_region_config(region)
        .context("Region not found in config")?
        .clone();

    let info_indices = config
        .get_valid_info_indices(region)
        .context("No valid info indices found")?;

    let mut thresholds = config.get_valid_thresholds(region).unwrap_or_default();

    // Load stock codes
    let stock_codes = load_stock_codes(stock_codes_path)?;

    // Create raw data directory
    let raw_data_dir = "raw_data";
    if !Path::new(raw_data_dir).exists() {
        fs::create_dir_all(raw_data_dir).context("Failed to create raw_data directory")?;
    }

    // Prepare database; load later based on user choice
    let mut database = StockDatabase::new(Vec::new());
    // Fixed subcontent top row used by legacy action screens
    let sub_top: u16 = 14;

    // Initial previous-data prompt shown below the main menu
    if let Some((latest_path, latest_name)) = find_latest_csv(raw_data_dir) {
        let mut out = std::io::stdout();
        out.queue(cursor::MoveTo(0, 0))?;
        out.queue(terminal::Clear(ClearType::All))?;
        use std::io::Write;
        write!(
            out,
            "Found previous data: {}. Load it? [y/N]: \r\n",
            latest_name
        )?;
        out.flush()?;

        // Temporarily disable raw to read input
        terminal::disable_raw_mode()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        terminal::enable_raw_mode()?;
        let choice = input.trim().to_lowercase();

        out.queue(cursor::MoveTo(0, sub_top))?;
        out.queue(terminal::Clear(ClearType::FromCursorDown))?;
        if choice == "y" {
            match StockDatabase::load_from_csv(latest_path.to_str().unwrap_or("")) {
                Ok(db) => {
                    database = db;
                    write!(out, "Data loaded from {}\r\n", latest_name)?;
                }
                Err(e) => {
                    write!(out, "Failed to load data: {}\r\n", e)?;
                }
            }
        } else {
            // User chose not to load previous data; fetch fresh data automatically
            out.flush()?;
            // Ensure screen below menu is clean before fetching
            drop(out);
            // Perform update with Ratatui progress
            let (data, saved_file) = run_fetch_progress(
                raw_data_dir,
                &stock_codes,
                region_config.clone(),
                info_indices.clone(),
            )
            .await?;
            database.update(data);
            println!("Saved: {}", saved_file);
        }
        // Nothing to redraw here; Ratatui UI will start below
    }

    // Main interactive loop using Ratatui
    loop {
        match run_main_menu()? {
            MenuAction::Update => {
                let (data, saved_file) = run_fetch_progress(
                    raw_data_dir,
                    &stock_codes,
                    region_config.clone(),
                    info_indices.clone(),
                )
                .await?;
                database.update(data);
                println!("Saved: {}", saved_file);
            }
            MenuAction::SetThresholds => {
                run_thresholds_editor(&mut thresholds)?;
            }
            MenuAction::Filter => {
                let codes = database.filter_stocks(&thresholds);
                run_results_table(&database, &codes)?;
            }
            MenuAction::Load => {
                if let Some(filename) = run_csv_picker(raw_data_dir)? {
                    match StockDatabase::load_from_csv(&filename) {
                        Ok(loaded_db) => {
                            database = loaded_db;
                            println!("Loaded: {}", filename);
                        }
                        Err(e) => {
                            eprintln!("Load failed for {}: {}", filename, e);
                        }
                    }
                }
            }
            MenuAction::Exit => break,
        }
    }

    Ok(())
}

fn load_stock_codes(file_path: &str) -> Result<Vec<String>> {
    if !Path::new(file_path).exists() {
        anyhow::bail!("Stock codes file not found: {}", file_path);
    }

    let mut reader = csv::Reader::from_path(file_path).context("Failed to open stock codes file")?;

    let mut codes = Vec::new();
    for result in reader.records() {
        let record = result.context("Failed to read CSV record")?;
        if let Some(code) = record.get(0) {
            codes.push(code.to_string());
        }
    }

    if codes.is_empty() {
        anyhow::bail!("Stock codes file is empty: {}", file_path);
    }

    Ok(codes)
}
