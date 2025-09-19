use anyhow::Result;
use crossterm::{execute, terminal};
use ratatui::{prelude::*, widgets::*};
use crate::database::StockDatabase;

pub fn run_results_table(database: &StockDatabase, codes: &[String]) -> Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut rows_data: Vec<&crate::fetcher::StockData> = Vec::new();
    for code in codes { if let Some(s) = database.data.iter().find(|s| &s.stock_code==code) { rows_data.push(s); } }
    let mut offset: usize = 0;

    loop {
        terminal.draw(|f| {
            let area_full = f.size();
            let chunks = Layout::default().direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(1)]).split(area_full);
            let table_area = chunks[0]; let footer_area = chunks[1];
            let capacity = (table_area.height.saturating_sub(3) as usize).max(1);
            let max_off = rows_data.len().saturating_sub(capacity);
            if offset > max_off { offset = max_off; }
            let header_cells = ["Stock Name","Code","Curr","Prev","Open","Inc","High","Low","Turn","Amp","TM"].into_iter().map(Cell::from);
            let header = Row::new(header_cells).style(Style::default().fg(Color::Yellow));
            let end = (offset + capacity).min(rows_data.len());
            let rows = rows_data[offset..end].iter().map(|s| Row::new(vec![
                Cell::from(s.stock_name.clone()), Cell::from(s.stock_code.clone()),
                Cell::from(format!("{:.2}", s.curr)), Cell::from(format!("{:.2}", s.prev_closed)),
                Cell::from(format!("{:.2}", s.open)), Cell::from(format!("{:.2}", s.increase)),
                Cell::from(format!("{:.2}", s.highest)), Cell::from(format!("{:.2}", s.lowest)),
                Cell::from(format!("{:.2}", s.turn_over)), Cell::from(format!("{:.2}", s.amp)), Cell::from(format!("{:.2}", s.tm)),
            ]));
            let table = Table::new(rows, [
                Constraint::Length(16), Constraint::Length(8), Constraint::Length(7), Constraint::Length(7),
                Constraint::Length(7), Constraint::Length(7), Constraint::Length(7), Constraint::Length(7),
                Constraint::Length(7), Constraint::Length(7), Constraint::Length(7),
            ]).header(header)
            .block(Block::default().borders(Borders::ALL).title(format!("Filtered Results ({} rows)", rows_data.len())))
            .column_spacing(1);
            f.render_widget(table, table_area);
            let footer_text = format!("Showing {}-{} of {}  •  ↑/↓/j/k scroll  •  PgUp/PgDn page  •  Home/End jump  •  Enter/Esc back",
                if rows_data.is_empty(){0}else{offset+1}, end, rows_data.len());
            f.render_widget(Paragraph::new(footer_text).style(Style::default().fg(Color::Gray)), footer_area);
        })?;

        if crossterm::event::poll(std::time::Duration::from_millis(200))? {
            if let crossterm::event::Event::Key(k) = crossterm::event::read()? {
                match k.code {
                    crossterm::event::KeyCode::Enter | crossterm::event::KeyCode::Esc => { terminal::disable_raw_mode()?; let mut out=std::io::stdout(); let _=execute!(out, terminal::LeaveAlternateScreen); return Ok(()); }
                    crossterm::event::KeyCode::Char('c') if k.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => { terminal::disable_raw_mode()?; let mut out=std::io::stdout(); let _=execute!(out, terminal::LeaveAlternateScreen); return Ok(()); }
                    crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => { offset = offset.saturating_add(1); }
                    crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => { offset = offset.saturating_sub(1); }
                    crossterm::event::KeyCode::PageDown => { let (_,h)=crossterm::terminal::size().unwrap_or((80,24)); let cap=h.saturating_sub(3) as usize; offset = offset.saturating_add(cap); }
                    crossterm::event::KeyCode::PageUp => { let (_,h)=crossterm::terminal::size().unwrap_or((80,24)); let cap=h.saturating_sub(3) as usize; offset = offset.saturating_sub(cap); }
                    crossterm::event::KeyCode::Home => { offset = 0; }
                    crossterm::event::KeyCode::End => { let (_,h)=crossterm::terminal::size().unwrap_or((80,24)); let cap=h.saturating_sub(3) as usize; let max_off = rows_data.len().saturating_sub(cap.max(1)); offset = max_off; }
                    _ => {}
                }
            }
        }
    }
}

