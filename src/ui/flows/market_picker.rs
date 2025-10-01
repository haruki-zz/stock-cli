use crate::error::{AppError, Result};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use std::time::Duration;

use crate::ui::styles::{header_text, secondary_line, selection_style};
use crate::ui::{TerminalGuard, UiRoute};

pub fn run_market_picker(options: &[(String, String)]) -> Result<String> {
    if options.is_empty() {
        return Err(AppError::message("No stock markets are available"));
    }

    let mut guard = TerminalGuard::new()?;
    let mut selected = 0usize;

    loop {
        guard.terminal_mut().draw(|f| {
            let size = f.size();
            let area = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(3),
                    Constraint::Length(1),
                ])
                .split(size);

            let title = Paragraph::new(header_text(UiRoute::MarketPicker.title()));
            f.render_widget(title, area[0]);

            let items: Vec<ListItem> = options
                .iter()
                .enumerate()
                .map(|(idx, (code, name))| {
                    let line = Line::from(vec![
                        Span::from(format!("{:<4}", code)).bold(),
                        "  ".into(),
                        Span::from(name.as_str()),
                    ]);
                    let mut item = ListItem::new(line);
                    if idx == selected {
                        item = item.style(selection_style());
                    }
                    item
                })
                .collect();

            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(UiRoute::MarketPicker.title()),
            );
            f.render_widget(list, area[1]);

            let help = Paragraph::new(secondary_line(
                "↑/↓ or j/k move • Enter select • Esc cancel • Ctrl+C exit",
            ));
            f.render_widget(help, area[2]);
        })?;

        if event::poll(Duration::from_millis(150))? {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        if selected == 0 {
                            selected = options.len() - 1;
                        } else {
                            selected -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        selected = (selected + 1) % options.len();
                    }
                    KeyCode::Enter => {
                        let choice = options[selected].0.clone();
                        guard.restore()?;
                        return Ok(choice);
                    }
                    KeyCode::Esc => {
                        guard.restore()?;
                        return Err(AppError::Cancelled);
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        guard.restore()?;
                        return Err(AppError::Cancelled);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}
