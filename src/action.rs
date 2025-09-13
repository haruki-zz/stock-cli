use anyhow::{Context, Result};
use chrono::Local;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};

use crate::config;
use crate::database::StockDatabase;
use crate::fetcher::{AsyncStockFetcher, StockData};
use crate::ui::menu_main::Menu;
use crate::ui::menu_sub_load_csv::choose_csv_file_interactively;
use crate::ui::menu_sub_threshold_setting::{display_thresholds, set_thresholds_interactively};
use crossterm::{cursor, terminal::{self, ClearType}, QueueableCommand};

pub fn render_main_menu_full(menu: &mut Menu) -> Result<()> {
    let mut out = std::io::stdout();
    out.queue(cursor::MoveTo(0, 0))?;
    out.queue(terminal::Clear(ClearType::All))?;
    out.flush()?;
    menu.show_banner()?;
    menu.display()?;
    Ok(())
}

pub fn pause_and_return(_sub_top: u16, menu: &mut Menu) -> Result<()> {
    // Temporarily disable raw, show prompt, wait for Enter, re-enable
    terminal::disable_raw_mode()?;
    print!("\nPress Enter to return to menu...");
    io::stdout().flush()?;
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    terminal::enable_raw_mode()?;

    // Clear the whole screen and redraw only the main menu
    render_main_menu_full(menu)?;
    Ok(())
}

pub fn find_latest_csv(dir: &str) -> Option<(std::path::PathBuf, String)> {
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

fn get_stock_codes_input() -> Result<Vec<String>> {
    print!("Enter stock codes (space-separated): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let codes: Vec<String> = input.trim().split_whitespace().map(|s| s.to_string()).collect();
    Ok(codes)
}

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

pub async fn do_update(
    database: &mut StockDatabase,
    menu: &mut Menu,
    raw_data_dir: &str,
    stock_codes: &[String],
    region_config: config::RegionConfig,
    info_indices: HashMap<String, config::InfoIndex>,
    sub_top: u16,
) -> Result<()> {
    // Clear sub area and run update
    {
        let mut out = std::io::stdout();
        out.queue(cursor::MoveTo(0, sub_top))?;
        out.queue(terminal::Clear(ClearType::FromCursorDown))?;
        out.flush()?;
    }
    {
        let mut out = std::io::stdout();
        use std::io::Write;
        write!(out, "Fetching latest data...\r\n")?;
        out.flush()?;
    }
    match fetch_new_data(
        raw_data_dir,
        stock_codes,
        region_config,
        info_indices,
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
                write!(out, "Update complete — ok: {}, failed: {} (total {})\r\n", succeeded, failed, total)?;
                out.flush()?;
            }
        }
        Err(e) => {
            use std::io::Write;
            let mut out = std::io::stdout();
            write!(out, "Update failed: {}\r\n", e)?;
            out.flush()?;
        }
    }
    pause_and_return(sub_top, menu)?;
    Ok(())
}

pub fn do_show(database: &StockDatabase, menu: &mut Menu, sub_top: u16) -> Result<()> {
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
        write!(out, "No stock codes entered.\r\n")?;
        out.flush()?;
    }
    pause_and_return(sub_top, menu)?;
    Ok(())
}

pub fn do_set_thresholds(
    thresholds: &mut HashMap<String, config::Threshold>,
    menu: &mut Menu,
    sub_top: u16,
) -> Result<()> {
    if let Err(e) = set_thresholds_interactively(thresholds, sub_top) {
        println!("Failed to set thresholds: {}", e);
    }
    // Clear entire screen so only main menu remains in control
    render_main_menu_full(menu)?;
    Ok(())
}

pub fn do_filter(
    database: &StockDatabase,
    thresholds: &HashMap<String, config::Threshold>,
    menu: &mut Menu,
    sub_top: u16,
) -> Result<()> {
    let mut out = std::io::stdout();
    out.queue(cursor::MoveTo(0, sub_top))?;
    out.queue(terminal::Clear(ClearType::FromCursorDown))?;
    out.flush()?;
    use std::io::Write;
    write!(out, "Current filters:\r\n")?;
    out.flush()?;
    display_thresholds(thresholds);
    let filtered_codes = database.filter_stocks(thresholds);
    if filtered_codes.is_empty() {
        write!(out, "No stocks match your filters.\r\n")?;
        out.flush()?;
    } else {
        write!(out, "Results:\r\n")?;
        out.flush()?;
        database.show_stock_info(&filtered_codes);
    }
    pause_and_return(sub_top, menu)?;
    Ok(())
}

pub fn do_load(
    database: &mut StockDatabase,
    menu: &mut Menu,
    raw_data_dir: &str,
    sub_top: u16,
) -> Result<()> {
    // Show interactive CSV selection list under raw_data
    let choice = choose_csv_file_interactively(raw_data_dir, sub_top)?;
    let mut out = std::io::stdout();
    out.queue(cursor::MoveTo(0, sub_top))?;
    out.queue(terminal::Clear(ClearType::FromCursorDown))?;
    out.flush()?;
    if let Some(filename) = choice {
        match StockDatabase::load_from_csv(&filename) {
            Ok(loaded_db) => {
                *database = loaded_db;
                menu.loaded_file = Some(filename.clone());
                use std::io::Write;
                write!(out, "Loaded: {}\r\n", filename)?;
                out.flush()?;
            }
            Err(e) => {
                use std::io::Write;
                write!(out, "Load failed for {}: {}\r\n", filename, e)?;
                out.flush()?;
            }
        }
    } else {
        use std::io::Write;
        write!(out, "Load canceled.\r\n")?;
        out.flush()?;
    }
    pause_and_return(sub_top, menu)?;
    Ok(())
}

