use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use std::time::Duration;

use crate::services::StockData;
use crate::storage::StockDatabase;
use crate::ui::{
    components::chart::{self, ChartState},
    TerminalGuard,
};

/// Display the filtered dataset with a movable cursor, summary panel, and optional chart.
pub fn run_results_table(database: &StockDatabase, codes: &[String]) -> Result<()> {
    let mut guard = TerminalGuard::new()?;

    let mut rows_data: Vec<&StockData> = Vec::new();
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
                chart_state.prepare_history(&stock.stock_code);
            } else {
                chart_state.clear_active();
            }
        }

        let mut capacity: usize = 1;

        let footer_height = if chart_state.show { 2 } else { 1 };

        guard.terminal_mut().draw(|f| {
            let area_full = f.size();
            let (list_area, chart_area) = if chart_state.show {
                let segments = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(40),
                        Constraint::Percentage(60),
                    ])
                    .split(area_full);
                (segments[0], Some(segments[1]))
            } else {
                (area_full, None)
            };

            let list_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Length(footer_height)])
                .split(list_area);
            let table_area = list_chunks[0];
            let footer_area = list_chunks[1];

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
                    let cells = vec![
                        Cell::from(stock.stock_name.clone()),
                        Cell::from(stock.stock_code.clone()),
                        Cell::from("│"),
                        Cell::from(format!("{:.2}", stock.curr)),
                        Cell::from(format!("{:.2}", stock.prev_closed)),
                        Cell::from(format!("{:.2}", stock.open)),
                        Cell::from(format!("{:.2}", stock.increase)),
                        Cell::from(format!("{:.2}", stock.highest)),
                        Cell::from(format!("{:.2}", stock.lowest)),
                        Cell::from(format!("{:.2}", stock.turn_over)),
                        Cell::from(format!("{:.2}", stock.amp)),
                        Cell::from(format!("{:.2}", stock.tm)),
                    ];
                    let mut row = Row::new(cells);
                    if offset + i == selected {
                        row = row.style(Style::default().add_modifier(Modifier::REVERSED));
                    }
                    row
                })
                .collect::<Vec<_>>();

            let header = Row::new(
                [
                    "Stock Name",
                    "Code",
                    "",
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
                Constraint::Length(14),
                Constraint::Length(8),
                Constraint::Length(3),
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

            let table = Table::new(base_rows, widths)
                .header(header)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                    .title(format!("Filtered Results ({} rows)", total)),
            )
            .column_spacing(0);
            f.render_widget(table, table_area);

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
        if footer_area.height > 0 {
            f.render_widget(
                Paragraph::new(footer_text)
                    .style(Style::default().fg(Color::Gray))
                    .wrap(Wrap { trim: true }),
                footer_area,
            );
        }

            if let Some(chart_area) = chart_area {
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
                                chart_state.prepare_history(&stock.stock_code);
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
                                    chart_state.prepare_history(&stock.stock_code);
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
                                    chart_state.prepare_history(&stock.stock_code);
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
                                        chart_state.prepare_history(&stock.stock_code);
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
                                        chart_state.prepare_history(&stock.stock_code);
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
                                    chart_state.prepare_history(&stock.stock_code);
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
                                    chart_state.prepare_history(&stock.stock_code);
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
