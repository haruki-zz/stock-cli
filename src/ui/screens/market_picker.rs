use anyhow::{anyhow, Result};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use std::time::Duration;

use crate::ui::TerminalGuard;

pub fn run_market_picker(options: &[(String, String)]) -> Result<String> {
    if options.is_empty() {
        return Err(anyhow!("No stock markets are available"));
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

            let title =
                Paragraph::new("Select a stock market")
                    .style(Style::default().fg(Color::Rgb(230, 121, 0)));
            f.render_widget(title, area[0]);

            let items: Vec<ListItem> = options
                .iter()
                .enumerate()
                .map(|(idx, (code, name))| {
                    let line = Line::from(vec![
                        Span::styled(
                            format!("{:<4}", code),
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("  "),
                        Span::raw(name.as_str()),
                    ]);
                    let mut item = ListItem::new(line);
                    if idx == selected {
                        item = item.style(Style::default().add_modifier(Modifier::REVERSED));
                    }
                    item
                })
                .collect();

            let list = List::new(items).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Markets (↑/↓ or j/k)"),
            );
            f.render_widget(list, area[1]);

            let help = Paragraph::new("Enter select • Esc cancel • Ctrl+C exit")
                .style(Style::default().fg(Color::Gray));
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
                        return Err(anyhow!("Market selection cancelled"));
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        guard.restore()?;
                        return Err(anyhow!("Market selection cancelled"));
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}
