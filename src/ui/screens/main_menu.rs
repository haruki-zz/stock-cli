use crate::error::Result;
use crate::ui::{components::utils::split_vertical, TerminalGuard};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::prelude::Stylize;
use ratatui::{prelude::*, widgets::*};
use std::time::Duration;

/// Logical actions triggered from the main menu screen.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    Update,
    Filter,
    Filters,
    Load,
    SwitchRegion,
    Exit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterMenuAction {
    Adjust,
    Save,
    Load,
    Back,
}

pub fn run_main_menu(
    loaded_file: Option<&str>,
    allow_region_switch: bool,
    region_code: &str,
    region_name: &str,
) -> Result<MenuAction> {
    // Ensure raw mode and the alternate screen are always restored regardless of how we exit.
    let mut guard = TerminalGuard::new()?;

    let mut items: Vec<(&str, &str, MenuAction)> = vec![
        (
            "Show Filtered",
            "Review the latest data using current filters",
            MenuAction::Filter,
        ),
        (
            "Refresh Data",
            "Fetch the newest stock snapshot",
            MenuAction::Update,
        ),
        (
            "Filters",
            "Adjust, save, or load threshold presets",
            MenuAction::Filters,
        ),
        (
            "Load CSV",
            "Pick a saved dataset from assets/snapshots/",
            MenuAction::Load,
        ),
    ];

    if allow_region_switch {
        items.push((
            "Switch Market",
            "Change the active market region",
            MenuAction::SwitchRegion,
        ));
    }

    items.push(("Quit", "Exit Stock CLI", MenuAction::Exit));
    let mut selected = 0usize;

    loop {
        guard.terminal_mut().draw(|f| {
            let size = f.size();
            let chunks = split_vertical(
                size,
                &[
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ],
            );

            let header_text = match loaded_file {
                Some(name) if !name.is_empty() => format!(
                    "Stock CLI — Main Menu\nRegion: {} — {}\nData file: {}",
                    region_code, region_name, name
                ),
                _ => format!(
                    "Stock CLI — Main Menu\nRegion: {} — {}\nData file: None",
                    region_code, region_name
                ),
            };
            let header = Paragraph::new(header_text).fg(Color::Rgb(230, 121, 0));
            f.render_widget(header, chunks[0]);

            let list_items: Vec<ListItem> = items
                .iter()
                .enumerate()
                .map(|(i, (label, description, _))| {
                    let line: Line = vec![
                        Span::from(format!("{:<18}", label)).bold(),
                        "  ".into(),
                        Span::from(*description).gray(),
                    ]
                    .into();
                    let item = ListItem::new(line);
                    if i == selected {
                        item.style(Style::default().add_modifier(Modifier::REVERSED))
                    } else {
                        item
                    }
                })
                .collect();
            let list = List::new(list_items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Menu (↑/↓ or j/k)"),
            );
            f.render_widget(list, chunks[1]);

            let help = Paragraph::new(
                "↑/↓ or j/k navigate • Enter select • Esc back • Ctrl+C exit".gray(),
            );
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
                        let action = items[selected].2.clone();
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

pub fn run_filters_menu() -> Result<FilterMenuAction> {
    let mut guard = TerminalGuard::new()?;
    let entries: Vec<(&str, &str, FilterMenuAction)> = vec![
        (
            "Set Filters",
            "Adjust threshold ranges",
            FilterMenuAction::Adjust,
        ),
        (
            "Save Filters",
            "Store current thresholds as a preset",
            FilterMenuAction::Save,
        ),
        (
            "Load Filters",
            "Apply a saved preset",
            FilterMenuAction::Load,
        ),
        ("Back", "Return to the main menu", FilterMenuAction::Back),
    ];
    let mut selected = 0usize;

    loop {
        guard.terminal_mut().draw(|f| {
            let size = f.size();
            let chunks = split_vertical(
                size,
                &[
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ],
            );

            let title =
                Paragraph::new("Filters — manage threshold presets").fg(Color::Rgb(230, 121, 0));
            f.render_widget(title, chunks[0]);

            let list_items: Vec<ListItem> = entries
                .iter()
                .enumerate()
                .map(|(i, (label, description, _))| {
                    let line: Line = vec![
                        Span::from(format!("{:<18}", label)).bold(),
                        "  ".into(),
                        Span::from(*description).gray(),
                    ]
                    .into();
                    let item = ListItem::new(line);
                    if i == selected {
                        item.style(Style::default().add_modifier(Modifier::REVERSED))
                    } else {
                        item
                    }
                })
                .collect();

            let list = List::new(list_items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Filters (↑/↓ or j/k)"),
            );
            f.render_widget(list, chunks[1]);

            let help = Paragraph::new("↑/↓ or j/k navigate • Enter select • Esc back".gray());
            f.render_widget(help, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(k) => match k.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if selected == 0 {
                            selected = entries.len() - 1;
                        } else {
                            selected -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        selected = (selected + 1) % entries.len();
                    }
                    KeyCode::Enter => {
                        let action = entries[selected].2.clone();
                        guard.restore()?;
                        return Ok(action);
                    }
                    KeyCode::Esc => {
                        guard.restore()?;
                        return Ok(FilterMenuAction::Back);
                    }
                    KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                        guard.restore()?;
                        return Ok(FilterMenuAction::Back);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}
