use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use std::time::Duration;

use crate::ui::TerminalGuard;

/// Logical actions triggered from the main menu screen.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    Update,
    SetThresholds,
    Filter,
    Load,
    Exit,
}

pub fn run_main_menu(loaded_file: Option<&str>) -> Result<MenuAction> {
    // Ensure raw mode and the alternate screen are always restored regardless of how we exit.
    let mut guard = TerminalGuard::new()?;

    let items: Vec<(&str, MenuAction)> = vec![
        ("Show Filtered", MenuAction::Filter),
        ("Set Filters", MenuAction::SetThresholds),
        ("Refresh Data", MenuAction::Update),
        ("Load CSV", MenuAction::Load),
        ("Quit", MenuAction::Exit),
    ];
    let mut selected = 0usize;

    loop {
        guard.terminal_mut().draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(size);

            let header_text = match loaded_file {
                Some(name) if !name.is_empty() => {
                    format!("Stock CLI — Main Menu\nData file: {}", name)
                }
                _ => "Stock CLI — Main Menu\nData file: None".to_string(),
            };
            let header = Paragraph::new(header_text).style(Style::default().fg(Color::Cyan));
            f.render_widget(header, chunks[0]);

            let list_items: Vec<ListItem> = items
                .iter()
                .enumerate()
                .map(|(i, (label, _))| {
                    let mut line = Line::from(*label);
                    if i == selected {
                        line = line.style(Style::default().add_modifier(Modifier::REVERSED));
                    }
                    ListItem::new(line)
                })
                .collect();
            let list =
                List::new(list_items).block(Block::default().borders(Borders::ALL).title("Menu"));
            f.render_widget(list, chunks[1]);

            let help = Paragraph::new("↑/↓ navigate • Enter select • Esc back • Ctrl+C exit")
                .style(Style::default().fg(Color::Gray));
            f.render_widget(help, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(k) => match k.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        // Wrap-around navigation keeps the UI snappy for keyboard users.
                        if selected == 0 {
                            selected = items.len() - 1;
                        } else {
                            selected -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        selected = (selected + 1) % items.len();
                    }
                    KeyCode::Enter => {
                        // Leave the alternate screen before returning so the caller can print freely.
                        let action = items[selected].1.clone();
                        guard.restore()?;
                        return Ok(action);
                    }
                    KeyCode::Esc => {
                        guard.restore()?;
                        return Ok(MenuAction::Exit);
                    }
                    KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                        guard.restore()?;
                        return Ok(MenuAction::Exit);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}
