use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::path::PathBuf;
use std::time::SystemTime;

/// Helper to carve out a centered rectangle sized by percentages of the parent area.
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    let vertical = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1]);
    vertical[1]
}

/// Return CSV files in descending modified order along with metadata for display.
pub fn list_csv_files(dir: &str) -> Vec<(String, PathBuf, SystemTime, u64)> {
    let mut entries = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("csv") {
                if let Ok(meta) = e.metadata() {
                    let m = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                    let s = meta.len();
                    if let Some(name) = p
                        .file_name()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                    {
                        entries.push((name, p, m, s));
                    }
                }
            }
        }
    }
    entries.sort_by(|a, b| b.2.cmp(&a.2));
    entries
}
