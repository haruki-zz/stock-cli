use anyhow::Result;
use chrono::{DateTime, Local};
use std::fs;
use std::path::PathBuf;

use crate::ui::navigation::navigate_list;
use crate::ui::select::{render_select_list, SelectItem};

fn list_csv_files(dir: &str) -> Vec<(String, PathBuf, std::time::SystemTime, u64)> {
    let mut entries: Vec<(String, PathBuf, std::time::SystemTime, u64)> = Vec::new();
    if let Ok(read_dir) = fs::read_dir(dir) {
        for e in read_dir.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("csv") {
                if let Ok(meta) = e.metadata() {
                    let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    let size = meta.len();
                    if let Some(name) = p.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()) {
                        entries.push((name, p, modified, size));
                    }
                }
            }
        }
    }
    // Sort by modified descending (newest first)
    entries.sort_by(|a, b| b.2.cmp(&a.2));
    entries
}

pub fn choose_csv_file_interactively(dir: &str, top_row: u16) -> Result<Option<String>> {
    use crossterm::{cursor, terminal::{self, ClearType}, QueueableCommand};
    use std::io::{stdout, Write};

    let mut stdout = stdout();

    let files = list_csv_files(dir);
    if files.is_empty() {
        // Show message and allow user to return
        stdout.queue(cursor::MoveTo(0, top_row))?;
        stdout.queue(terminal::Clear(ClearType::FromCursorDown))?;
        stdout.write_all(b"No CSV files in raw_data/. Use 'Refresh Data' to fetch.\r\n")?;
        stdout.flush()?;
        return Ok(None);
    }

    // Build menu items with filename and description (modified time + size)
    let items: Vec<SelectItem> = files
        .iter()
        .map(|(name, _p, modified, size)| {
            let dt: DateTime<Local> = (*modified).into();
            let kb = (*size as f64) / 1024.0;
            SelectItem {
                label: name.clone(),
                description: format!("modified {}  —  {:.1} KB", dt.format("%Y-%m-%d %H:%M"), kb),
            }
        })
        .collect();

    let render = |sel: usize| -> Result<()> {
        // Clear and render title + list
        stdout.queue(cursor::MoveTo(0, top_row))?;
        stdout.queue(terminal::Clear(ClearType::FromCursorDown))?;
        stdout.write_all(b"Select a CSV to load \xE2\x80\x94 \xE2\x86\x91/\xE2\x86\x93 navigate, Enter select, Esc cancel\r\n\r\n")?;
        let list_top = top_row + 2;
        render_select_list(list_top, &items, sel)?;
        Ok(())
    };

    let total = || items.len();
    match navigate_list(0, total, render)? {
        Some(sel) => Ok(Some(files[sel].1.to_string_lossy().to_string())),
        None => Ok(None),
    }
}
