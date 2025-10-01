use crate::error::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::widgets::*;
use std::time::Duration;

use crate::ui::styles::{secondary_line, selection_style};
use crate::ui::{TerminalGuard, UiRoute};
use crate::utils::{format_file_modified, list_csv_files};
use ratatui::text::Line as TextLine;

pub fn run_csv_picker(dir: &str) -> Result<Option<String>> {
    // Protect terminal state while the picker owns the screen.
    let mut guard = TerminalGuard::new()?;

    let files = list_csv_files(dir);
    if files.is_empty() {
        // Surface a transient message instead of leaving the user on a blank screen.
        guard.terminal_mut().draw(|f| {
            let size = f.size();
            let block = Paragraph::new(secondary_line(
                "No CSV files in assets/snapshots/. Use 'Refresh Data' to fetch.",
            ))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(UiRoute::CsvPicker.title()),
            );
            f.render_widget(block, size);
        })?;
        std::thread::sleep(std::time::Duration::from_millis(1200));
        guard.restore()?;
        return Ok(None);
    }

    let mut selected = 0usize;
    loop {
        guard.terminal_mut().draw(|f| {
            let size = f.size();
            let items: Vec<ListItem> = files
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    let kb = (entry.size as f64) / 1024.0;
                    let text = format!(
                        "{}  —  {}  —  {:.1} KB",
                        entry.name,
                        format_file_modified(entry.modified),
                        kb
                    );
                    let item = ListItem::new(TextLine::from(text));
                    if i == selected {
                        item.style(selection_style())
                    } else {
                        item
                    }
                })
                .collect();
            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Choose CSV — \u{2191}/\u{2193}/j/k move, Enter select, Esc cancel"),
            );
            f.render_widget(list, size);
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(k) = event::read()? {
                match k.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        // Allow wrap-around navigation for quick top/bottom access.
                        if selected == 0 {
                            selected = files.len() - 1;
                        } else {
                            selected -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        selected = (selected + 1) % files.len();
                    }
                    KeyCode::Enter => {
                        let path = files[selected].path.clone();
                        guard.restore()?;
                        return Ok(Some(path.to_string_lossy().to_string()));
                    }
                    KeyCode::Esc => {
                        guard.restore()?;
                        return Ok(None);
                    }
                    KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                        guard.restore()?;
                        return Ok(None);
                    }
                    _ => {}
                }
            }
        }
    }
}
