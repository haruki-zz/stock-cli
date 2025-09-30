use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use std::time::Duration;

use crate::ui::{components::utils::list_csv_files, TerminalGuard};

pub fn run_csv_picker(dir: &str) -> Result<Option<String>> {
    // Protect terminal state while the picker owns the screen.
    let mut guard = TerminalGuard::new()?;

    let files = list_csv_files(dir);
    if files.is_empty() {
        // Surface a transient message instead of leaving the user on a blank screen.
        guard.terminal_mut().draw(|f| {
            let size = f.size();
            let block = Paragraph::new(
                "No CSV files in assets/snapshots/. Use 'Refresh Data' to fetch."
            )
                .block(Block::default().borders(Borders::ALL).title("Load CSV"));
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
                .map(|(i, (name, _p, m, sz))| {
                    let dt: chrono::DateTime<chrono::Local> = (*m).into();
                    let kb = (*sz as f64) / 1024.0;
                    let text = format!(
                        "{}  —  {}  —  {:.1} KB",
                        name,
                        dt.format("%Y-%m-%d %H:%M"),
                        kb
                    );
                    let mut line = Line::from(text);
                    if i == selected {
                        line = line.style(Style::default().add_modifier(Modifier::REVERSED));
                    }
                    ListItem::new(line)
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
                        let path = files[selected].1.to_string_lossy().to_string();
                        guard.restore()?;
                        return Ok(Some(path));
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
