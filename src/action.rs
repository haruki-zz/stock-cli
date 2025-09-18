use anyhow::Result;
use std::fs;
use std::io;

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

// (View Stocks) feature removed.
