use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use std::time::Duration;

use crate::ui::{components::utils::list_json_files, TerminalGuard};

pub fn run_preset_picker(dir: &str) -> Result<Option<String>> {
    let mut guard = TerminalGuard::new()?;

    let files = list_json_files(dir);
    if files.is_empty() {
        guard.terminal_mut().draw(|f| {
            let size = f.size();
            let block = Paragraph::new("No saved filters found. Use 'Save Filters' first.")
                .block(Block::default().borders(Borders::ALL).title("Load Filters"));
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
                .map(|(i, (name, _path, modified, size_bytes))| {
                    let dt: chrono::DateTime<chrono::Local> = (*modified).into();
                    let kb = (*size_bytes as f64) / 1024.0;
                    let text = format!(
                        "{:<24}  {}  {:>6.1} KB",
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
                    .title("Choose Filter Preset — ↑/↓/j/k move, Enter select, Esc cancel"),
            );
            f.render_widget(list, size);
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(k) = event::read()? {
                match k.code {
                    KeyCode::Up | KeyCode::Char('k') => {
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
