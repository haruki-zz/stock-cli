use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use std::time::Duration;

use crate::{database::StockDatabase, ui::TerminalGuard};

pub fn run_results_table(database: &StockDatabase, codes: &[String]) -> Result<()> {
    let mut guard = TerminalGuard::new()?;

    let mut rows_data: Vec<&crate::fetcher::StockData> = Vec::new();
    for code in codes {
        if let Some(stock) = database.data.iter().find(|s| &s.stock_code == code) {
            rows_data.push(stock);
        }
    }

    let mut offset: usize = 0;
    let mut selected: usize = 0;

    loop {
        let mut capacity: usize = 1;
        let mut visible_end: usize = 0;

        guard.terminal_mut().draw(|f| {
            let area_full = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .split(area_full);
            let table_area = chunks[0];
            let detail_area = chunks[1];
            let footer_area = chunks[2];

            capacity = (table_area.height.saturating_sub(3) as usize).max(1);
            let total = rows_data.len();
            if total == 0 {
                selected = 0;
            } else if selected >= total {
                selected = total - 1;
            }

            let max_offset = total.saturating_sub(capacity);
            if offset > max_offset {
                offset = max_offset;
            }

            let header_cells = [
                "Stock Name",
                "Code",
                "Curr",
                "Prev",
                "Open",
                "Inc",
                "High",
                "Low",
                "Turn",
                "Amp",
                "TM",
            ]
            .into_iter()
            .map(Cell::from);
            let header = Row::new(header_cells).style(Style::default().fg(Color::Yellow));

            visible_end = (offset + capacity).min(total);
            let rows = rows_data[offset..visible_end]
                .iter()
                .enumerate()
                .map(|(i, stock)| {
                    let mut row = Row::new(vec![
                        Cell::from(stock.stock_name.clone()),
                        Cell::from(stock.stock_code.clone()),
                        Cell::from(format!("{:.2}", stock.curr)),
                        Cell::from(format!("{:.2}", stock.prev_closed)),
                        Cell::from(format!("{:.2}", stock.open)),
                        Cell::from(format!("{:.2}", stock.increase)),
                        Cell::from(format!("{:.2}", stock.highest)),
                        Cell::from(format!("{:.2}", stock.lowest)),
                        Cell::from(format!("{:.2}", stock.turn_over)),
                        Cell::from(format!("{:.2}", stock.amp)),
                        Cell::from(format!("{:.2}", stock.tm)),
                    ]);
                    if offset + i == selected {
                        row = row.style(Style::default().add_modifier(Modifier::REVERSED));
                    }
                    row
                });

            let table = Table::new(
                rows,
                [
                    Constraint::Length(16),
                    Constraint::Length(8),
                    Constraint::Length(7),
                    Constraint::Length(7),
                    Constraint::Length(7),
                    Constraint::Length(7),
                    Constraint::Length(7),
                    Constraint::Length(7),
                    Constraint::Length(7),
                    Constraint::Length(7),
                    Constraint::Length(7),
                ],
            )
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Filtered Results ({} rows)", total)),
            )
            .column_spacing(1);
            f.render_widget(table, table_area);

            let detail_text = rows_data
                .get(selected)
                .map(|stock| {
                    format!(
                        "{} ({})  curr: {:.2}  inc: {:.2}  high/low: {:.2}/{:.2}  turnover: {:.2}  amp: {:.2}  tm: {:.2}",
                        stock.stock_name,
                        stock.stock_code,
                        stock.curr,
                        stock.increase,
                        stock.highest,
                        stock.lowest,
                        stock.turn_over,
                        stock.amp,
                        stock.tm
                    )
                })
                .unwrap_or_else(|| "No rows".to_string());
            let detail = Paragraph::new(detail_text)
                .block(Block::default().borders(Borders::ALL).title("Selected"));
            f.render_widget(detail, detail_area);

            let footer_text = if total == 0 {
                "No rows • Esc/Enter back".to_string()
            } else {
                format!(
                    "Row {}/{}  •  Showing {}-{} of {}  •  ↑/↓ move  •  PgUp/PgDn page  •  Home/End jump  •  Esc/Enter back",
                    selected + 1,
                    total,
                    offset + 1,
                    visible_end,
                    total
                )
            };
            f.render_widget(
                Paragraph::new(footer_text).style(Style::default().fg(Color::Gray)),
                footer_area,
            );
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(k) = event::read()? {
                let total = rows_data.len();
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
                        if total > 0 && selected + 1 < total {
                            selected += 1;
                            if selected >= offset + capacity {
                                offset = selected + 1 - capacity;
                            }
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if total > 0 && selected > 0 {
                            selected -= 1;
                            if selected < offset {
                                offset = selected;
                            }
                        }
                    }
                    KeyCode::PageDown => {
                        if total > 0 {
                            let new_selected = (selected + capacity).min(total.saturating_sub(1));
                            selected = new_selected;
                            if selected >= offset + capacity {
                                offset = selected + 1 - capacity;
                            }
                        }
                    }
                    KeyCode::PageUp => {
                        if total > 0 {
                            let new_selected = selected.saturating_sub(capacity);
                            selected = new_selected;
                            if selected < offset {
                                offset = selected;
                            }
                        }
                    }
                    KeyCode::Home => {
                        if total > 0 {
                            selected = 0;
                            offset = 0;
                        }
                    }
                    KeyCode::End => {
                        if total > 0 {
                            selected = total - 1;
                            offset = selected.saturating_sub(capacity.saturating_sub(1));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
