use anyhow::Result;
use crossterm::{execute, terminal};
use ratatui::{prelude::*, widgets::*};

use crate::ui::menu_main::MenuAction;

pub fn run_main_menu(loaded_file: Option<&str>) -> Result<MenuAction> {
    terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let items: Vec<(&str, MenuAction)> = vec![
        ("Show Filtered", MenuAction::Filter),
        ("Set Filters", MenuAction::SetThresholds),
        ("Refresh Data", MenuAction::Update),
        ("Load CSV", MenuAction::Load),
        ("Quit", MenuAction::Exit),
    ];
    let mut selected = 0usize;

    loop {
        terminal.draw(|f| {
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
                Some(name) if !name.is_empty() => format!("Stock CLI — Main Menu\nData file: {}", name),
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
            let list = List::new(list_items).block(Block::default().borders(Borders::ALL).title("Menu"));
            f.render_widget(list, chunks[1]);

            let help = Paragraph::new("↑/↓ navigate • Enter select • Esc back • Ctrl+C exit").style(Style::default().fg(Color::Gray));
            f.render_widget(help, chunks[2]);
        })?;

        if crossterm::event::poll(std::time::Duration::from_millis(200))? {
            match crossterm::event::read()? {
                crossterm::event::Event::Key(k) => match k.code {
                    crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                        if selected == 0 { selected = items.len() - 1; } else { selected -= 1; }
                    }
                    crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                        selected = (selected + 1) % items.len();
                    }
                    crossterm::event::KeyCode::Enter => {
                        let action = items[selected].1.clone();
                        terminal::disable_raw_mode()?;
                        let mut out = std::io::stdout();
                        let _ = execute!(out, terminal::LeaveAlternateScreen);
                        return Ok(action);
                    }
                    crossterm::event::KeyCode::Esc | crossterm::event::KeyCode::Char('c') if k.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        terminal::disable_raw_mode()?;
                        let mut out = std::io::stdout();
                        let _ = execute!(out, terminal::LeaveAlternateScreen);
                        return Ok(MenuAction::Exit);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

