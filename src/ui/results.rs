use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use std::time::Duration;

use crate::database::StockDatabase;
use crate::ui::{
    chart::{self, ChartState},
    TerminalGuard,
};

/// Display the filtered dataset with a movable cursor, summary panel, and optional chart.
pub fn run_results_table(
    database: &StockDatabase,
    codes: &[String],
    raw_data_dir: &str,
) -> Result<()> {
    let mut guard = TerminalGuard::new()?;

    let mut rows_data: Vec<&crate::fetcher::StockData> = Vec::new();
    for code in codes {
        if let Some(stock) = database.data.iter().find(|s| &s.stock_code == code) {
            rows_data.push(stock);
        }
    }

    let mut offset: usize = 0;
    let mut selected: usize = 0;
    let mut chart_state = ChartState::default();

    loop {
        if chart_state.show {
            if let Some(stock) = rows_data.get(selected) {
                chart_state.prepare_history(raw_data_dir, &stock.stock_code);
            } else {
                chart_state.clear_active();
            }
        }

        let mut capacity: usize = 1;

        let footer_height = if chart_state.show { 2 } else { 1 };

        guard.terminal_mut().draw(|f| {
            let area_full = f.size();
            let left_pct = if chart_state.show { 40 } else { 100 };
            let right_pct = 100 - left_pct;
            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(left_pct),
                    Constraint::Percentage(right_pct),
                ])
                .split(area_full);

            let left_area = columns[0];
            let right_area = if chart_state.show && right_pct > 0 {
                Some(columns[1])
            } else {
                None
            };

            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),
                    Constraint::Length(3),
                    Constraint::Length(footer_height),
                ])
                .split(left_area);
            let table_area = left_chunks[0];
            let detail_area = left_chunks[1];
            let footer_area = left_chunks[2];

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

            let visible_end = (offset + capacity).min(total);
            let base_rows = rows_data[offset..visible_end]
                .iter()
                .enumerate()
                .map(|(i, stock)| {
                    let cells = if chart_state.show {
                        vec![
                            Cell::from(stock.stock_name.clone()),
                            Cell::from(stock.stock_code.clone()),
                        ]
                    } else {
                        vec![
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
                        ]
                    };
                    let mut row = Row::new(cells);
                    if offset + i == selected {
                        row = row.style(Style::default().add_modifier(Modifier::REVERSED));
                    }
                    row
                })
                .collect::<Vec<_>>();

            let (header, widths, rows_iter) = if chart_state.show {
                let header = Row::new(vec![Cell::from("Stock Name"), Cell::from("Code")])
                    .style(Style::default().fg(Color::Yellow));
                let widths = vec![Constraint::Length(22), Constraint::Length(12)];
                (header, widths, base_rows)
            } else {
                let header = Row::new(
                    [
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
                    .map(Cell::from)
                    .collect::<Vec<_>>(),
                )
                .style(Style::default().fg(Color::Yellow));
                let widths = vec![
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
                ];
                (header, widths, base_rows)
            };

            let table = Table::new(rows_iter, widths)
                .header(header)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                    .title(format!("Filtered Results ({} rows)", total)),
            )
            .column_spacing(1);
            f.render_widget(table, table_area);

            if chart_state.show {
                let detail_text = rows_data
                    .get(selected)
                    .map(|stock| {
                        format!(
                            "Curr: {:.2}  Prev: {:.2}  Open: {:.2}  Inc: {:.2}\nHigh: {:.2}  Low: {:.2}  Turn: {:.2}  Amp: {:.2}  TM: {:.2}",
                            stock.curr,
                            stock.prev_closed,
                            stock.open,
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
            }

        let footer_text = if total == 0 {
            "No rows • Esc back".to_string()
        } else if chart_state.show {
            format!(
                "Row {}/{} • {}-{} of {} • ↑/↓ move • PgUp/PgDn page • Home/End jump • Enter/←/→ timeframe • X close • Esc back",
                selected + 1,
                total,
                offset + 1,
                visible_end,
                total
            )
        } else {
            format!(
                "Row {}/{} • {}-{} of {} • ↑/↓ move • PgUp/PgDn page • Home/End jump • Enter chart • Esc back",
                selected + 1,
                total,
                offset + 1,
                visible_end,
                total
            )
        };
        // Render footer outside the frame border on the terminal's last line.
        if footer_area.height > 0 {
            let last_line_y = area_full.y + area_full.height.saturating_sub(1);
            let last_line_area = Rect {
                x: area_full.x,
                y: last_line_y,
                width: area_full.width,
                height: 1,
            };
            f.render_widget(
                Paragraph::new(footer_text).style(Style::default().fg(Color::Gray)),
                last_line_area,
            );
        }

            if let Some(chart_area) = right_area {
                let selected_stock = rows_data.get(selected).copied();
                chart::render_chart_panel(
                    f,
                    chart_area,
                    footer_height as u16,
                    &chart_state,
                    selected_stock,
                );
            }
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(k) = event::read()? {
                let total = rows_data.len();
                match k.code {
                    KeyCode::Enter => {
                        if !chart_state.show {
                            if let Some(stock) = rows_data.get(selected) {
                                chart_state.show = true;
                                chart_state.timeframe_index = 0;
                                chart_state.prepare_history(raw_data_dir, &stock.stock_code);
                            }
                        } else {
                            chart_state.next_timeframe();
                        }
                    }
                    KeyCode::Esc => {
                        guard.restore()?;
                        return Ok(());
                    }
                    KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                        guard.restore()?;
                        return Ok(());
                    }
                    KeyCode::Char('x') => {
                        chart_state.hide();
                    }
                    KeyCode::Right => {
                        if chart_state.show {
                            chart_state.next_timeframe();
                        }
                    }
                    KeyCode::Left => {
                        if chart_state.show {
                            chart_state.prev_timeframe();
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if total > 0 {
                            selected = (selected + 1) % total;
                            if selected >= offset + capacity {
                                offset = selected + 1 - capacity;
                            } else if selected < offset {
                                offset = selected;
                            }
                            if chart_state.show {
                                if let Some(stock) = rows_data.get(selected) {
                                    chart_state.prepare_history(raw_data_dir, &stock.stock_code);
                                }
                            }
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if total > 0 {
                            selected = selected.checked_sub(1).unwrap_or(total - 1);
                            if selected < offset {
                                offset = selected;
                            } else if selected >= offset + capacity {
                                offset = selected + 1 - capacity;
                            }
                            if chart_state.show {
                                if let Some(stock) = rows_data.get(selected) {
                                    chart_state.prepare_history(raw_data_dir, &stock.stock_code);
                                }
                            }
                        }
                    }
                    KeyCode::PageDown => {
                        if total > 0 {
                            let new_selected = (selected + capacity).min(total.saturating_sub(1));
                            if new_selected != selected {
                                selected = new_selected;
                                if selected >= offset + capacity {
                                    offset = selected + 1 - capacity;
                                }
                                if chart_state.show {
                                    if let Some(stock) = rows_data.get(selected) {
                                        chart_state
                                            .prepare_history(raw_data_dir, &stock.stock_code);
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::PageUp => {
                        if total > 0 {
                            let new_selected = selected.saturating_sub(capacity);
                            if new_selected != selected {
                                selected = new_selected;
                                if selected < offset {
                                    offset = selected;
                                }
                                if chart_state.show {
                                    if let Some(stock) = rows_data.get(selected) {
                                        chart_state
                                            .prepare_history(raw_data_dir, &stock.stock_code);
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Home => {
                        if total > 0 {
                            selected = 0;
                            offset = 0;
                            if chart_state.show {
                                if let Some(stock) = rows_data.get(selected) {
                                    chart_state.prepare_history(raw_data_dir, &stock.stock_code);
                                }
                            }
                        }
                    }
                    KeyCode::End => {
                        if total > 0 {
                            selected = total - 1;
                            offset = selected.saturating_sub(capacity.saturating_sub(1));
                            if chart_state.show {
                                if let Some(stock) = rows_data.get(selected) {
                                    chart_state.prepare_history(raw_data_dir, &stock.stock_code);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
