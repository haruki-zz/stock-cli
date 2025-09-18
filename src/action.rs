use anyhow::Result;
use std::fs;
use std::io::{self, Write};

use crate::database::StockDatabase;
use crossterm::terminal;

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

pub fn do_show(database: &StockDatabase) -> Result<()> {
    terminal::disable_raw_mode()?;
    let codes = get_stock_codes_input()?;
    terminal::enable_raw_mode()?;
    let mut out = std::io::stdout();
    if !codes.is_empty() {
        database.show_stock_info(&codes);
    } else {
        use std::io::Write;
        write!(out, "No stock codes entered.\r\n")?;
        out.flush()?;
    }
    // Simple pause before returning to the UI
    terminal::disable_raw_mode()?;
    print!("\nPress Enter to return to menu...");
    io::stdout().flush()?;
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input);
    terminal::enable_raw_mode()?;
    Ok(())
}
