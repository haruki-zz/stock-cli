use anyhow::Result;
use crossterm::{event, execute, terminal};
use ratatui::{prelude::*, widgets::*};

use crate::ui::menu_main::MenuAction;
use crate::config::{RegionConfig, InfoIndex};
use crate::fetcher::{AsyncStockFetcher, StockData};

pub struct AppState<'a> {
    pub items: Vec<(&'a str, MenuAction)>,
    pub selected: usize,
}

impl<'a> AppState<'a> {
    fn new() -> Self {
        Self {
            items: vec![
                ("Show Filtered", MenuAction::Filter),
                ("Set Filters", MenuAction::SetThresholds),
                ("Refresh Data", MenuAction::Update),
                ("View Stocks", MenuAction::Show),
                ("Load CSV", MenuAction::Load),
                ("Quit", MenuAction::Exit),
            ],
            selected: 0,
        }
    }
}

pub fn run_main_menu() -> Result<MenuAction> {
    // Setup terminal for Ratatui
    terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut app = AppState::new();

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

            // Header
            let header = Paragraph::new("Stock CLI — Main Menu")
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(header, chunks[0]);

            // Menu list
            let items: Vec<ListItem> = app
                .items
                .iter()
                .enumerate()
                .map(|(i, (label, _))| {
                    let mut line = Line::from(*label);
                    if i == app.selected {
                        line = line.style(Style::default().add_modifier(Modifier::REVERSED));
                    }
                    ListItem::new(line)
                })
                .collect();
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Menu"));
            f.render_widget(list, chunks[1]);

            // Help
            let help = Paragraph::new("↑/↓ navigate • Enter select • Esc back • Ctrl+C exit")
                .style(Style::default().fg(Color::Gray));
            f.render_widget(help, chunks[2]);
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            match event::read()? {
                event::Event::Key(k) => match k.code {
                    event::KeyCode::Up | event::KeyCode::Char('k') => {
                        if app.selected == 0 {
                            app.selected = app.items.len() - 1;
                        } else {
                            app.selected -= 1;
                        }
                    }
                    event::KeyCode::Down | event::KeyCode::Char('j') => {
                        app.selected = (app.selected + 1) % app.items.len();
                    }
                    event::KeyCode::Enter => {
                        let action = app.items[app.selected].1.clone();
                        // Tear down terminal and return action for caller to perform
                        terminal::disable_raw_mode()?;
                        let mut out = std::io::stdout();
                        let _ = execute!(out, terminal::LeaveAlternateScreen);
                        return Ok(action);
                    }
                    event::KeyCode::Esc => {
                        terminal::disable_raw_mode()?;
                        let mut out = std::io::stdout();
                        let _ = execute!(out, terminal::LeaveAlternateScreen);
                        return Ok(MenuAction::Exit);
                    }
                    event::KeyCode::Char('c') if k.modifiers.contains(event::KeyModifiers::CONTROL) => {
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

fn list_csv_files(dir: &str) -> Vec<(String, std::path::PathBuf, std::time::SystemTime, u64)> {
    let mut entries = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("csv") {
                if let Ok(meta) = e.metadata() {
                    let m = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    let s = meta.len();
                    if let Some(name) = p.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()) {
                        entries.push((name, p, m, s));
                    }
                }
            }
        }
    }
    entries.sort_by(|a, b| b.2.cmp(&a.2));
    entries
}

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
                .block(Block::default().borders(Borders::ALL).title("Choose CSV (Enter to select, Esc to cancel)"));
            f.render_widget(list, size);
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let event::Event::Key(k) = event::read()? {
                match k.code {
                    event::KeyCode::Up | event::KeyCode::Char('k') => {
                        if selected == 0 { selected = files.len() - 1; } else { selected -= 1; }
                    }
                    event::KeyCode::Down | event::KeyCode::Char('j') => {
                        selected = (selected + 1) % files.len();
                    }
                    event::KeyCode::Enter => {
                        let path = files[selected].1.to_string_lossy().to_string();
                        terminal::disable_raw_mode()?;
                        let mut out = std::io::stdout();
                        let _ = execute!(out, terminal::LeaveAlternateScreen);
                        return Ok(Some(path));
                    }
                    event::KeyCode::Esc => {
                        terminal::disable_raw_mode()?;
                        let mut out = std::io::stdout();
                        let _ = execute!(out, terminal::LeaveAlternateScreen);
                        return Ok(None);
                    }
                    event::KeyCode::Char('c') if k.modifiers.contains(event::KeyModifiers::CONTROL) => {
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

use crate::config::Threshold;
use crate::database::StockDatabase;

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

pub fn run_thresholds_editor(thresholds: &mut std::collections::HashMap<String, Threshold>) -> Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut keys: Vec<String> = thresholds.keys().cloned().collect();
    keys.sort();
    let mut selected = 0usize;

    #[derive(Clone)]
    enum Mode {
        List,
        Edit { name: String, lower: String, upper: String, field: usize, orig_lower: f64, orig_upper: f64 }, // field: 0 lower, 1 upper
        AddName { name: String },
        AddValues { name: String, lower: String, upper: String, field: usize },
    }
    let mut mode = Mode::List;

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(2), Constraint::Min(1), Constraint::Length(1)])
                .split(size);

            let title = Paragraph::new("Edit thresholds — \u{2191}/\u{2193} navigate, Enter edit, Esc back")
                .style(Style::default().fg(Color::Cyan));
            f.render_widget(title, chunks[0]);

            // List view
            let mut items_vec: Vec<ListItem> = keys
                .iter()
                .map(|k| {
                    let thr = thresholds.get(k).unwrap();
                    let text = format!("{:<12}  [{:>6.2}, {:>6.2}]", k, thr.lower, thr.upper);
                    ListItem::new(text)
                })
                .collect();
            items_vec.push(ListItem::new("Add filter"));
            items_vec.push(ListItem::new("Back"));
            let list = List::new(
                items_vec
                    .into_iter()
                    .enumerate()
                    .map(|(i, mut it)| {
                        if i == selected {
                            it = it.style(Style::default().add_modifier(Modifier::REVERSED));
                        }
                        it
                    })
                    .collect::<Vec<_>>(),
            )
            .block(Block::default().borders(Borders::ALL).title("Thresholds"));
            f.render_widget(list, chunks[1]);

            let help = Paragraph::new("Enter edit • Esc back • Tab switch field in editor")
                .style(Style::default().fg(Color::Gray));
            f.render_widget(help, chunks[2]);

            // Modal editor/add dialogs
            match &mode {
                Mode::Edit { name, lower, upper, field, .. } => {
                    let area = centered_rect(60, 40, size);
                    let block = Block::default().borders(Borders::ALL).title(format!("Edit '{}'", name));
                    f.render_widget(Clear, area);
                    f.render_widget(block.clone(), area);
                    let inner = block.inner(area);
                    let v = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
                        .split(inner);
                    let l_style = if *field == 0 { Style::default().add_modifier(Modifier::REVERSED) } else { Style::default() };
                    let u_style = if *field == 1 { Style::default().add_modifier(Modifier::REVERSED) } else { Style::default() };
                    f.render_widget(Paragraph::new(format!("Lower: {}", lower)).style(l_style), v[0]);
                    f.render_widget(Paragraph::new(format!("Upper: {}", upper)).style(u_style), v[1]);
                    f.render_widget(Paragraph::new("Enter save • Esc cancel • Tab/↑/↓/j/k switch").style(Style::default().fg(Color::Gray)), v[2]);
                }
                Mode::AddName { name } => {
                    let area = centered_rect(60, 30, size);
                    f.render_widget(Clear, area);
                    let block = Block::default().borders(Borders::ALL).title("New metric name");
                    f.render_widget(block.clone(), area);
                    let inner = block.inner(area);
                    f.render_widget(Paragraph::new(format!("Name: {}", name)), inner);
                }
                Mode::AddValues { name, lower, upper, field } => {
                    let area = centered_rect(60, 40, size);
                    f.render_widget(Clear, area);
                    let block = Block::default().borders(Borders::ALL).title(format!("Add '{}'", name));
                    f.render_widget(block.clone(), area);
                    let inner = block.inner(area);
                    let v = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
                        .split(inner);
                    let l_style = if *field == 0 { Style::default().add_modifier(Modifier::REVERSED) } else { Style::default() };
                    let u_style = if *field == 1 { Style::default().add_modifier(Modifier::REVERSED) } else { Style::default() };
                    f.render_widget(Paragraph::new(format!("Lower: {}", lower)).style(l_style), v[0]);
                    f.render_widget(Paragraph::new(format!("Upper: {}", upper)).style(u_style), v[1]);
                    f.render_widget(Paragraph::new("Enter save • Esc cancel • Tab/↑/↓/j/k switch").style(Style::default().fg(Color::Gray)), v[2]);
                }
                Mode::List => {}
            }
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let event::Event::Key(k) = event::read()? {
                match (&mode, k.code) {
                    (Mode::List, event::KeyCode::Up) | (Mode::List, event::KeyCode::Char('k')) => {
                        if selected == 0 { selected = keys.len() + 1; } else { selected -= 1; }
                    }
                    (Mode::List, event::KeyCode::Down) | (Mode::List, event::KeyCode::Char('j')) => { selected = (selected + 1) % (keys.len() + 2); }
                    (Mode::List, event::KeyCode::Enter) => {
                        if selected < keys.len() {
                            let name = keys[selected].clone();
                            if let Some(existing) = thresholds.get(&name) {
                                // Start with empty input so user can type new value immediately; keep originals for fallback
                                mode = Mode::Edit { name, lower: String::new(), upper: String::new(), field: 0, orig_lower: existing.lower, orig_upper: existing.upper };
                            }
                        } else if selected == keys.len() {
                            mode = Mode::AddName { name: String::new() };
                        } else {
                            terminal::disable_raw_mode()?;
                            let mut out = std::io::stdout();
                            let _ = execute!(out, terminal::LeaveAlternateScreen);
                            return Ok(());
                        }
                    }
                    (Mode::List, event::KeyCode::Esc) => {
                        terminal::disable_raw_mode()?;
                        let mut out = std::io::stdout();
                        let _ = execute!(out, terminal::LeaveAlternateScreen);
                        return Ok(());
                    }
                    // Editor input handling
                    (Mode::Edit { name, lower, upper, field, orig_lower, orig_upper }, key) => {
                        let mut nm = name.clone();
                        let mut lo = lower.clone();
                        let mut up = upper.clone();
                        let mut fld = *field;
                        match key {
                            event::KeyCode::Tab | event::KeyCode::Down | event::KeyCode::Char('j') => { fld = (fld + 1) % 2; }
                            event::KeyCode::Up | event::KeyCode::Char('k') => { fld = (fld + 1) % 2; }
                            event::KeyCode::Backspace => {
                                if fld == 0 { lo.pop(); } else { up.pop(); }
                            }
                            event::KeyCode::Char(c) if c.is_ascii_digit() || c == '.' || c == '-' => {
                                if fld == 0 { lo.push(c); } else { up.push(c); }
                            }
                            event::KeyCode::Enter => {
                                let lo_v = if lo.is_empty() { *orig_lower } else { lo.parse::<f64>().unwrap_or(*orig_lower) };
                                let up_v = if up.is_empty() { *orig_upper } else { up.parse::<f64>().unwrap_or(*orig_upper) };
                                let (lo_v, up_v) = if lo_v <= up_v { (lo_v, up_v) } else { (up_v, lo_v) };
                                thresholds.insert(nm.clone(), Threshold { lower: lo_v, upper: up_v, valid: true });
                                mode = Mode::List;
                                continue;
                            }
                            event::KeyCode::Esc => { mode = Mode::List; continue; }
                            _ => {}
                        }
                        mode = Mode::Edit { name: nm, lower: lo, upper: up, field: fld, orig_lower: *orig_lower, orig_upper: *orig_upper };
                    }
                    (Mode::AddName { name }, key) => {
                        let mut nm = name.clone();
                        match key {
                            event::KeyCode::Backspace => { nm.pop(); }
                            event::KeyCode::Char(c) => { if !c.is_control() { nm.push(c); } }
                            event::KeyCode::Enter => {
                                if !nm.is_empty() {
                                    mode = Mode::AddValues { name: nm.clone(), lower: String::new(), upper: String::new(), field: 0 };
                                } else { mode = Mode::List; }
                                continue;
                            }
                            event::KeyCode::Esc => { mode = Mode::List; continue; }
                            _ => {}
                        }
                        mode = Mode::AddName { name: nm };
                    }
                    (Mode::AddValues { name, lower, upper, field }, key) => {
                        let mut nm = name.clone();
                        let mut lo = lower.clone();
                        let mut up = upper.clone();
                        let mut fld = *field;
                        match key {
                            event::KeyCode::Tab | event::KeyCode::Down | event::KeyCode::Char('j') => { fld = (fld + 1) % 2; }
                            event::KeyCode::Up | event::KeyCode::Char('k') => { fld = (fld + 1) % 2; }
                            event::KeyCode::Backspace => { if fld == 0 { lo.pop(); } else { up.pop(); } }
                            event::KeyCode::Char(c) if c.is_ascii_digit() || c == '.' || c == '-' => { if fld == 0 { lo.push(c); } else { up.push(c); } }
                            event::KeyCode::Enter => {
                                let lo_v = lo.parse::<f64>().unwrap_or(0.0);
                                let up_v = up.parse::<f64>().unwrap_or(lo_v);
                                let (lo_v, up_v) = if lo_v <= up_v { (lo_v, up_v) } else { (up_v, lo_v) };
                                thresholds.insert(nm.clone(), Threshold { lower: lo_v, upper: up_v, valid: true });
                                if !keys.contains(&nm) { keys.push(nm.clone()); keys.sort(); }
                                mode = Mode::List; continue;
                            }
                            event::KeyCode::Esc => { mode = Mode::List; continue; }
                            _ => {}
                        }
                        mode = Mode::AddValues { name: nm, lower: lo, upper: up, field: fld };
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn run_results_table(database: &StockDatabase, codes: &[String]) -> Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    // Build rows from database for given codes
    let mut rows_data: Vec<&crate::fetcher::StockData> = Vec::new();
    for code in codes {
        if let Some(s) = database.data.iter().find(|s| &s.stock_code == code) {
            rows_data.push(s);
        }
    }

    let mut offset: usize = 0;

    loop {
        terminal.draw(|f| {
            let area_full = f.size();
            // Reserve one line for footer/status
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(1)])
                .split(area_full);

            let table_area = chunks[0];
            let footer_area = chunks[1];

            // Estimate visible row capacity: subtract borders (2) and header (1)
            let capacity = (table_area
                .height
                .saturating_sub(3) as usize)
                .max(1);

            // Clamp offset
            let max_off = rows_data.len().saturating_sub(capacity);
            if offset > max_off { offset = max_off; }

            let header_cells = [
                "Stock Name","Code","Curr","Prev","Open","Inc","High","Low","Turn","Amp","TM"
            ].into_iter().map(|h| Cell::from(h));
            let header = Row::new(header_cells).style(Style::default().fg(Color::Yellow)).bottom_margin(0);

            let end = (offset + capacity).min(rows_data.len());
            let rows = rows_data[offset..end].iter().map(|s| {
                Row::new(vec![
                    Cell::from(s.stock_name.clone()),
                    Cell::from(s.stock_code.clone()),
                    Cell::from(format!("{:.2}", s.curr)),
                    Cell::from(format!("{:.2}", s.prev_closed)),
                    Cell::from(format!("{:.2}", s.open)),
                    Cell::from(format!("{:.2}", s.increase)),
                    Cell::from(format!("{:.2}", s.highest)),
                    Cell::from(format!("{:.2}", s.lowest)),
                    Cell::from(format!("{:.2}", s.turn_over)),
                    Cell::from(format!("{:.2}", s.amp)),
                    Cell::from(format!("{:.2}", s.tm)),
                ])
            });

            let table = Table::new(
                rows,
                [
                    Constraint::Length(16), Constraint::Length(8), Constraint::Length(7), Constraint::Length(7),
                    Constraint::Length(7), Constraint::Length(7), Constraint::Length(7), Constraint::Length(7),
                    Constraint::Length(7), Constraint::Length(7), Constraint::Length(7),
                ],
            )
            .header(header)
            .block(Block::default().borders(Borders::ALL).title(format!("Filtered Results ({} rows)", rows_data.len())))
            .column_spacing(1);
            f.render_widget(table, table_area);

            // Footer with paging info and controls
            let footer_text = format!(
                "Showing {}-{} of {}  •  ↑/↓ or j/k scroll  •  PgUp/PgDn page  •  Home/End jump  •  Enter/Esc back",
                if rows_data.is_empty() { 0 } else { offset + 1 },
                end,
                rows_data.len()
            );
            f.render_widget(
                Paragraph::new(footer_text).style(Style::default().fg(Color::Gray)),
                footer_area,
            );
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let event::Event::Key(k) = event::read()? {
                match k.code {
                    event::KeyCode::Enter | event::KeyCode::Esc => {
                        terminal::disable_raw_mode()?;
                        let mut out = std::io::stdout();
                        let _ = execute!(out, terminal::LeaveAlternateScreen);
                        return Ok(());
                    }
                    event::KeyCode::Char('c') if k.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        terminal::disable_raw_mode()?;
                        let mut out = std::io::stdout();
                        let _ = execute!(out, terminal::LeaveAlternateScreen);
                        return Ok(());
                    }
                    event::KeyCode::Down | event::KeyCode::Char('j') => {
                        offset = offset.saturating_add(1);
                    }
                    event::KeyCode::Up | event::KeyCode::Char('k') => {
                        offset = offset.saturating_sub(1);
                    }
                    event::KeyCode::PageDown => {
                        // Recompute capacity based on last draw? approximate via terminal.size()
                        let (_, h) = crossterm::terminal::size().unwrap_or((80, 24));
                        let cap = h.saturating_sub(3) as usize;
                        offset = offset.saturating_add(cap);
                    }
                    event::KeyCode::PageUp => {
                        let (_, h) = crossterm::terminal::size().unwrap_or((80, 24));
                        let cap = h.saturating_sub(3) as usize;
                        offset = offset.saturating_sub(cap);
                    }
                    event::KeyCode::Home => { offset = 0; }
                    event::KeyCode::End => {
                        // Need total and capacity to compute max offset; approximate via terminal size
                        let (_, h) = crossterm::terminal::size().unwrap_or((80, 24));
                        let cap = h.saturating_sub(3) as usize;
                        let max_off = rows_data.len().saturating_sub(cap.max(1));
                        offset = max_off;
                    }
                    _ => {}
                }
            }
        }
    }
}

pub async fn run_fetch_progress(
    raw_data_dir: &str,
    stock_codes: &[String],
    region_config: RegionConfig,
    info_indices: std::collections::HashMap<String, InfoIndex>,
) -> Result<(Vec<StockData>, String)> {
    // Prepare fetcher and spawn background task
    let fetcher = AsyncStockFetcher::new(stock_codes.to_vec(), region_config, info_indices);
    let progress = fetcher.progress_counter.clone();
    let total = fetcher.total_stocks;

    let handle = tokio::spawn(async move { fetcher.fetch_data().await });

    // Setup Ratatui progress UI
    terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    loop {
        // Check if task finished without blocking the UI
        if handle.is_finished() {
            break;
        }

        let done = progress.load(std::sync::atomic::Ordering::SeqCst);
        let ratio = if total == 0 { 0.0 } else { (done as f64 / total as f64).clamp(0.0, 1.0) };

        terminal.draw(|f| {
            let size = f.size();
            let area = centered_rect(60, 20, size);
            f.render_widget(Clear, area);
            let block = Block::default().borders(Borders::ALL).title("Fetching latest data...");
            f.render_widget(block.clone(), area);
            let inner = block.inner(area);
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(inner);
            let label = format!("Progress: {} / {} ({:.0}%)", done.min(total), total, ratio * 100.0);
            f.render_widget(Paragraph::new("Please wait while we fetch data").alignment(Alignment::Center), chunks[0]);
            f.render_widget(Paragraph::new(label).alignment(Alignment::Center), chunks[1]);
            f.render_widget(Paragraph::new("Esc to cancel").style(Style::default().fg(Color::Gray)).alignment(Alignment::Center), chunks[2]);
        })?;

        // Allow cancel
        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(k) = crossterm::event::read()? {
                if matches!(k.code, crossterm::event::KeyCode::Esc)
                    || (k.code == crossterm::event::KeyCode::Char('c') && k.modifiers.contains(crossterm::event::KeyModifiers::CONTROL))
                {
                    // Best-effort cancel: we cannot easily cancel reqwest in-flight, so just wait for it to finish
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    }

    // Fetch result
    let res = handle.await.expect("fetch task join");
    let data = res?;

    // Save CSV similar to action.rs fetch_new_data
    let timestamp = chrono::Local::now().format("%Y_%m_%d_%H_%M");
    let database = StockDatabase::new(data.clone());
    let filename = format!("{}/{}_raw.csv", raw_data_dir, timestamp);
    database.save_to_csv(&filename)?;

    // Final success screen
    terminal.draw(|f| {
        let size = f.size();
        let area = centered_rect(60, 20, size);
        f.render_widget(Clear, area);
        let block = Block::default().borders(Borders::ALL).title("Done");
        f.render_widget(block.clone(), area);
        let inner = block.inner(area);
        let msg = Paragraph::new(format!("Fetched {} records. Saved to {}\nPress Enter to continue.", data.len(), filename))
            .alignment(Alignment::Center);
        f.render_widget(msg, inner);
    })?;

    // Wait for Enter
    loop {
        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(k) = crossterm::event::read()? {
                if matches!(k.code, crossterm::event::KeyCode::Enter | crossterm::event::KeyCode::Char('\n') | crossterm::event::KeyCode::Char('\r')) {
                    break;
                }
            }
        }
    }

    // Cleanup terminal
    terminal::disable_raw_mode()?;
    let mut out = std::io::stdout();
    let _ = execute!(out, terminal::LeaveAlternateScreen);

    Ok((data, filename))
}
