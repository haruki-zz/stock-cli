use anyhow::{Context, Result};
use chrono::Local;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::config::{self, Config};
use crate::database::StockDatabase;
use crate::fetcher::{AsyncStockFetcher, StockData};
use crate::menu::{Menu, MenuAction};
use crate::threshold_menu::{display_thresholds, set_thresholds_interactively};
use crossterm::{cursor, terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen}, ExecutableCommand, QueueableCommand};

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

    // Enter shared alternate screen + raw mode once
    {
        let mut out = std::io::stdout();
        out.execute(EnterAlternateScreen)?;
        terminal::enable_raw_mode()?;
    }

    // Render main menu at top (full-screen clear once)
    let mut menu = Menu::new();
    render_main_menu_full(&mut menu)?;

    // Compute subcontent top row (below menu)
    let sub_top: u16 = {
        let menu_rows = menu.items.len() as u16;
        // banner: BANNER_HEIGHT (10) + gap (1); menu starts at 11, so sub_top after menu + one blank line
        10 + 1 + menu_rows + 2
    };

    // Initial previous-data prompt shown below the main menu
    if let Some((latest_path, latest_name)) = find_latest_csv(raw_data_dir) {
        let mut out = std::io::stdout();
        out.queue(cursor::MoveTo(0, sub_top))?;
        out.queue(terminal::Clear(ClearType::FromCursorDown))?;
        use std::io::Write;
        write!(
            out,
            "Previous data detected. Load data from latest file {}? (y/n): \r\n",
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
                    menu.loaded_file = Some(latest_name.clone());
                    write!(out, "Data loaded from {}\r\n", latest_name)?;
                }
                Err(e) => {
                    write!(out, "Failed to load data: {}\r\n", e)?;
                }
            }
        } else {
            write!(out, "Skipping loading previous data.\r\n")?;
        }
        out.flush()?;

        // Redraw clean main menu
        render_main_menu_full(&mut menu)?;
    }

    // Main interactive loop sharing the same screen
    loop {
        // Ensure raw mode is enabled before capturing navigation input
        let _ = terminal::enable_raw_mode();
        let action = menu.choose_action()?;

        match action {
            MenuAction::Update => {
                // Clear sub area and run update
                {
                    let mut out = std::io::stdout();
                    out.queue(cursor::MoveTo(0, sub_top))?;
                    out.queue(terminal::Clear(ClearType::FromCursorDown))?;
                    out.flush()?;
                }
                {
                    use std::io::Write;
                    let mut out = std::io::stdout();
                    write!(out, "Fetching new data...\r\n")?;
                    out.flush()?;
                }
                match fetch_new_data(
                    raw_data_dir,
                    &stock_codes,
                    region_config.clone(),
                    info_indices.clone(),
                )
                .await
                {
                    Ok((new_data, saved_file)) => {
                        database.update(new_data);
                        menu.loaded_file = Some(saved_file);
                        let succeeded = database.data.len();
                        let total = stock_codes.len();
                        let failed = total.saturating_sub(succeeded);
                        {
                            use std::io::Write;
                            let mut out = std::io::stdout();
                            write!(out, "Stock information updated successfully.\r\n")?;
                            write!(out, "Succeeded: {}  Failed: {} (Total: {})\r\n", succeeded, failed, total)?;
                            out.flush()?;
                        }
                    }
                    Err(e) => {
                        use std::io::Write;
                        let mut out = std::io::stdout();
                        write!(out, "Failed to update stock information: {}\r\n", e)?;
                        out.flush()?;
                    }
                }
                pause_and_return(sub_top, &mut menu)?;
            }
            MenuAction::Show => {
                // Disable raw to accept line input, then re-enable
                terminal::disable_raw_mode()?;
                let codes = get_stock_codes_input()?;
                terminal::enable_raw_mode()?;
                let mut out = std::io::stdout();
                out.queue(cursor::MoveTo(0, sub_top))?;
                out.queue(terminal::Clear(ClearType::FromCursorDown))?;
                out.flush()?;
                if !codes.is_empty() {
                    database.show_stock_info(&codes);
                } else {
                    use std::io::Write;
                    let mut out = std::io::stdout();
                    write!(out, "No stock codes entered.\r\n")?;
                    out.flush()?;
                }
                pause_and_return(sub_top, &mut menu)?;
            }
            MenuAction::SetThresholds => {
                if let Err(e) = set_thresholds_interactively(&mut thresholds, sub_top) {
                    println!("Failed to set thresholds: {}", e);
                }
                // Clear entire screen so only main menu remains in control
                render_main_menu_full(&mut menu)?;
            }
            MenuAction::Filter => {
                let mut out = std::io::stdout();
                out.queue(cursor::MoveTo(0, sub_top))?;
                out.queue(terminal::Clear(ClearType::FromCursorDown))?;
                out.flush()?;
                use std::io::Write;
                write!(out, "Filtering stocks with thresholds (valid only):\r\n")?;
                out.flush()?;
                display_thresholds(&thresholds);
                let filtered_codes = database.filter_stocks(&thresholds);
                if filtered_codes.is_empty() {
                    write!(out, "No stocks match the filtering criteria.\r\n")?;
                    out.flush()?;
                } else {
                    write!(out, "Filtering results:\r\n")?;
                    out.flush()?;
                    database.show_stock_info(&filtered_codes);
                }
                pause_and_return(sub_top, &mut menu)?;
            }
            MenuAction::Load => {
                terminal::disable_raw_mode()?;
                let filename = get_filename_input()?;
                terminal::enable_raw_mode()?;
                let mut out = std::io::stdout();
                out.queue(cursor::MoveTo(0, sub_top))?;
                out.queue(terminal::Clear(ClearType::FromCursorDown))?;
                out.flush()?;
                if !filename.is_empty() {
                    match StockDatabase::load_from_csv(&filename) {
                        Ok(loaded_db) => {
                            database = loaded_db;
                            menu.loaded_file = Some(filename.clone());
                            use std::io::Write;
                            let mut out = std::io::stdout();
                            write!(out, "Data loaded from {}\r\n", filename)?;
                            out.flush()?;
                        }
                        Err(e) => {
                            use std::io::Write;
                            let mut out = std::io::stdout();
                            write!(out, "Failed to load data from {}: {}\r\n", filename, e)?;
                            out.flush()?;
                        }
                    }
                } else {
                    use std::io::Write;
                    let mut out = std::io::stdout();
                    write!(out, "No filename entered.\r\n")?;
                    out.flush()?;
                }
                pause_and_return(sub_top, &mut menu)?;
            }
            MenuAction::Exit => {
                println!("Exiting...");
                break;
            }
        }
    }

    // Cleanup screen
    {
        let mut out = std::io::stdout();
        let _ = terminal::disable_raw_mode();
        let _ = out.execute(LeaveAlternateScreen);
    }

    Ok(())
}

fn get_stock_codes_input() -> Result<Vec<String>> {
    print!("Enter stock codes (separated by spaces): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let codes: Vec<String> = input.trim().split_whitespace().map(|s| s.to_string()).collect();

    Ok(codes)
}

fn get_filename_input() -> Result<String> {
    print!("Enter filename: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}

fn pause_and_return(_sub_top: u16, menu: &mut Menu) -> Result<()> {
    // Temporarily disable raw, show prompt, wait for Enter, re-enable
    terminal::disable_raw_mode()?;
    print!("\nPress Enter to return...");
    io::stdout().flush()?;
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    terminal::enable_raw_mode()?;

    // Clear the whole screen and redraw only the main menu
    render_main_menu_full(menu)?;
    Ok(())
}

fn render_main_menu_full(menu: &mut Menu) -> Result<()> {
    let mut out = std::io::stdout();
    out.queue(cursor::MoveTo(0, 0))?;
    out.queue(terminal::Clear(ClearType::All))?;
    out.flush()?;
    menu.show_banner()?;
    menu.display()?;
    Ok(())
}

// (legacy) load_or_fetch_data removed; initialization flow now prompts below the main menu.

async fn fetch_new_data(
    raw_data_dir: &str,
    stock_codes: &[String],
    region_config: config::RegionConfig,
    info_indices: HashMap<String, config::InfoIndex>,
) -> Result<(Vec<StockData>, String)> {
    let timestamp = Local::now().format("%Y_%m_%d_%H_%M");

    let fetcher = AsyncStockFetcher::new(stock_codes.to_vec(), region_config, info_indices);

    let data = fetcher
        .fetch_data()
        .await
        .context("Failed to fetch stock data")?;

    let database = StockDatabase::new(data.clone());
    let filename = format!("{}/{}_raw.csv", raw_data_dir, timestamp);
    database
        .save_to_csv(&filename)
        .context("Failed to save data to CSV")?;

    Ok((data, filename))
}

fn find_latest_csv(dir: &str) -> Option<(std::path::PathBuf, String)> {
    let entries = fs::read_dir(dir).ok()?;
    let mut latest: Option<(std::time::SystemTime, std::path::PathBuf)> = None;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("csv") {
            if let Ok(meta) = e.metadata() {
                if let Ok(modified) = meta.modified() {
                    if latest.as_ref().map(|(t, _)| &modified > t).unwrap_or(true) {
                        latest = Some((modified, p));
                    }
                }
            }
        }
    }
    latest.map(|(_, p)| {
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
        (p, name)
    })
}

fn get_default_stock_codes() -> Vec<String> {
    vec![
        "sh000001",
        "sz399001",
        "sh600000",
        "sz000001",
        "sh600036",
        "sz000002",
        "sh600519",
        "sh601318",
        "sz300059",
        "sh688981",
    ]
    .into_iter()
    .map(String::from)
    .collect()
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
            println!(
                "Using default stock codes (couldn't create file: {})",
                file_path
            );
        }

        return Ok(sample_codes);
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
        println!("Empty stock codes file, using defaults");
        return Ok(get_default_stock_codes());
    }

    Ok(codes)
}
