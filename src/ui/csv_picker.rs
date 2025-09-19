use anyhow::Result;
use crossterm::{execute, terminal};
use ratatui::{prelude::*, widgets::*};

use crate::ui::utils::list_csv_files;

pub fn run_csv_picker(dir: &str) -> Result<Option<String>> {
    terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let files = list_csv_files(dir);
    if files.is_empty() {
        terminal.draw(|f| {
            let size = f.size();
            let block = Paragraph::new("No CSV files in raw_data/. Use 'Refresh Data' to fetch.")
                .block(Block::default().borders(Borders::ALL).title("Load CSV"));
            f.render_widget(block, size);
        })?;
        std::thread::sleep(std::time::Duration::from_millis(1200));
        terminal::disable_raw_mode()?;
        let mut out = std::io::stdout();
        let _ = execute!(out, terminal::LeaveAlternateScreen);
        return Ok(None);
    }

    let mut selected = 0usize;
    loop {
        terminal.draw(|f| {
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
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Choose CSV — \u{2191}/\u{2193}/j/k move, Enter select, Esc cancel"),
                );
            f.render_widget(list, size);
        })?;

        if crossterm::event::poll(std::time::Duration::from_millis(200))? {
            if let crossterm::event::Event::Key(k) = crossterm::event::read()? {
                match k.code {
                    crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                        if selected == 0 { selected = files.len() - 1; } else { selected -= 1; }
                    }
                    crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                        selected = (selected + 1) % files.len();
                    }
                    crossterm::event::KeyCode::Enter => {
                        let path = files[selected].1.to_string_lossy().to_string();
                        terminal::disable_raw_mode()?;
                        let mut out = std::io::stdout();
                        let _ = execute!(out, terminal::LeaveAlternateScreen);
                        return Ok(Some(path));
                    }
                    crossterm::event::KeyCode::Esc => {
                        terminal::disable_raw_mode()?;
                        let mut out = std::io::stdout();
                        let _ = execute!(out, terminal::LeaveAlternateScreen);
                        return Ok(None);
                    }
                    crossterm::event::KeyCode::Char('c') if k.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        terminal::disable_raw_mode()?;
                        let mut out = std::io::stdout();
                        let _ = execute!(out, terminal::LeaveAlternateScreen);
                        return Ok(None);
                    }
                    _ => {}
                }
            }
        }
    }
}

