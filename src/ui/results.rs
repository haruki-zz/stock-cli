use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal,
};
use ratatui::{prelude::*, widgets::*};
use std::time::Duration;

use crate::{database::StockDatabase, ui::TerminalGuard};

pub fn run_results_table(database: &StockDatabase, codes: &[String]) -> Result<()> {
    let mut guard = TerminalGuard::new()?;

    let mut rows_data: Vec<&crate::fetcher::StockData> = Vec::new();
    for code in codes {
        if let Some(s) = database.data.iter().find(|s| &s.stock_code == code) {
            rows_data.push(s);
        }
    }
    let mut offset: usize = 0;

    loop {
        guard.terminal_mut().draw(|f| {
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

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(k) = event::read()? {
                match k.code {
                    KeyCode::Enter | KeyCode::Esc => {
                        guard.restore()?;
                        return Ok(());
                    }
                    KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                        guard.restore()?;
                        return Ok(());
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        offset = offset.saturating_add(1);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        offset = offset.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        let (_, h) = terminal::size().unwrap_or((80, 24));
                        let cap = h.saturating_sub(3) as usize;
                        offset = offset.saturating_add(cap);
                    }
                    KeyCode::PageUp => {
                        let (_, h) = terminal::size().unwrap_or((80, 24));
                        let cap = h.saturating_sub(3) as usize;
                        offset = offset.saturating_sub(cap);
                    }
                    KeyCode::Home => {
                        offset = 0;
                    }
                    KeyCode::End => {
                        let (_, h) = terminal::size().unwrap_or((80, 24));
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
