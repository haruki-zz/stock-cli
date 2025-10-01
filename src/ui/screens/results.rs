use crate::error::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use std::{convert::TryFrom, time::Duration};
use unicode_width::UnicodeWidthStr;

use crate::fetch::StockData;
use crate::records::StockDatabase;
use crate::ui::{
    components::chart::{self, ChartState},
    TerminalGuard,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SortField {
    LastPrice,
    PrevClose,
    OpenPrice,
    Change,
    DayHigh,
    DayLow,
    Turnover,
    Amplitude,
    TotalMarket,
}

impl SortField {
    const ALL: [SortField; 9] = [
        SortField::LastPrice,
        SortField::PrevClose,
        SortField::OpenPrice,
        SortField::Change,
        SortField::DayHigh,
        SortField::DayLow,
        SortField::Turnover,
        SortField::Amplitude,
        SortField::TotalMarket,
    ];

    fn label(self) -> &'static str {
        match self {
            SortField::LastPrice => "Last Price",
            SortField::PrevClose => "Prev Close",
            SortField::OpenPrice => "Open Price",
            SortField::Change => "Change (%)",
            SortField::DayHigh => "Day High",
            SortField::DayLow => "Day Low",
            SortField::Turnover => "Turnover",
            SortField::Amplitude => "Amplitude",
            SortField::TotalMarket => "Total Market",
        }
    }

    fn next(self) -> Self {
        let idx = Self::ALL
            .iter()
            .position(|field| *field == self)
            .unwrap_or(0);
        let next_idx = (idx + 1) % Self::ALL.len();
        Self::ALL[next_idx]
    }

    fn compare(self, a: &StockData, b: &StockData) -> std::cmp::Ordering {
        match self {
            SortField::LastPrice => cmp_f64(a.curr, b.curr),
            SortField::PrevClose => cmp_f64(a.prev_closed, b.prev_closed),
            SortField::OpenPrice => cmp_f64(a.open, b.open),
            SortField::Change => cmp_f64(a.increase, b.increase),
            SortField::DayHigh => cmp_f64(a.highest, b.highest),
            SortField::DayLow => cmp_f64(a.lowest, b.lowest),
            SortField::Turnover => cmp_f64(a.turn_over, b.turn_over),
            SortField::Amplitude => cmp_f64(a.amp, b.amp),
            SortField::TotalMarket => cmp_f64(a.tm, b.tm),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SortState {
    field: SortField,
    descending: bool,
}

impl SortState {
    fn new() -> Self {
        Self {
            field: SortField::LastPrice,
            descending: true,
        }
    }

    fn cycle_field(&mut self) {
        self.field = self.field.next();
    }

    fn toggle_direction(&mut self) {
        self.descending = !self.descending;
    }

    fn direction_icon(self) -> &'static str {
        if self.descending {
            "↓"
        } else {
            "↑"
        }
    }
}

fn cmp_f64(a: f64, b: f64) -> std::cmp::Ordering {
    a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
}

fn build_sorted_rows<'a>(
    database: &'a StockDatabase,
    codes: &[String],
    sort: SortState,
) -> Vec<&'a StockData> {
    let mut rows: Vec<&StockData> = codes
        .iter()
        .filter_map(|code| database.data.iter().find(|s| &s.stock_code == code))
        .collect();

    rows.sort_by(|a, b| {
        use std::cmp::Ordering;

        let primary = sort.field.compare(a, b);
        let ord = if primary == Ordering::Equal {
            a.stock_code.cmp(&b.stock_code)
        } else {
            primary
        };
        if sort.descending {
            ord.reverse()
        } else {
            ord
        }
    });

    rows
}

fn rebuild_sorted_rows<'a>(
    database: &'a StockDatabase,
    codes: &[String],
    sort_state: SortState,
    current_code: Option<String>,
    selected: &mut usize,
    offset: &mut usize,
    capacity: usize,
    chart_state: &mut ChartState,
) -> Vec<&'a StockData> {
    let rows = build_sorted_rows(database, codes, sort_state);

    if let Some(code) = current_code {
        if let Some(idx) = rows.iter().position(|s| s.stock_code == code) {
            *selected = idx;
        } else if !rows.is_empty() {
            *selected = (*selected).min(rows.len() - 1);
        } else {
            *selected = 0;
        }
    } else if !rows.is_empty() {
        *selected = (*selected).min(rows.len() - 1);
    } else {
        *selected = 0;
    }

    if *selected >= *offset + capacity {
        *offset = selected.saturating_sub(capacity.saturating_sub(1));
    }
    if *selected < *offset {
        *offset = *selected;
    }

    if chart_state.show {
        if let Some(stock) = rows.get(*selected) {
            chart_state.prepare_history(&stock.stock_code, &stock.market);
        } else {
            chart_state.clear_active();
        }
    }

    rows
}

/// Display the filtered dataset with a movable cursor, summary panel, and optional chart.
pub fn run_results_table(database: &StockDatabase, codes: &[String]) -> Result<()> {
    let mut guard = TerminalGuard::new()?;

    let mut sort_state = SortState::new();
    let mut rows_data = build_sorted_rows(database, codes, sort_state);

    let mut offset: usize = 0;
    let mut selected: usize = 0;
    let mut chart_state = ChartState::default();

    loop {
        if chart_state.show {
            if let Some(stock) = rows_data.get(selected) {
                chart_state.prepare_history(&stock.stock_code, &stock.market);
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

            let header_columns: [(&str, Option<SortField>); 12] = [
                ("Stock Name", None),
                ("Code", None),
                ("", None),
                ("Last Price", Some(SortField::LastPrice)),
                ("Prev Close", Some(SortField::PrevClose)),
                ("Open Price", Some(SortField::OpenPrice)),
                ("Change (%)", Some(SortField::Change)),
                ("Day High", Some(SortField::DayHigh)),
                ("Day Low", Some(SortField::DayLow)),
                ("Turnover", Some(SortField::Turnover)),
                ("Amplitude", Some(SortField::Amplitude)),
                ("Total Market", Some(SortField::TotalMarket)),
            ];

            let header_cells: Vec<Cell> = header_columns
                .iter()
                .map(|(label, field)| {
                    let mut content = (*label).to_string();
                    if field.map(|f| f == sort_state.field).unwrap_or(false) {
                        content.push(' ');
                        content.push_str(sort_state.direction_icon());
                        Cell::from(content)
                            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    } else {
                        Cell::from(content).style(Style::default().fg(Color::Yellow))
                    }
                })
                .collect();

            let header = Row::new(header_cells);

            let name_header_width = UnicodeWidthStr::width(header_columns[0].0);
            let code_header_width = UnicodeWidthStr::width(header_columns[1].0);
            let name_data_width = rows_data
                .iter()
                .map(|stock| UnicodeWidthStr::width(stock.stock_name.as_str()))
                .max()
                .unwrap_or(0);
            let code_data_width = rows_data
                .iter()
                .map(|stock| UnicodeWidthStr::width(stock.stock_code.as_str()))
                .max()
                .unwrap_or(0);

            let name_col_width = u16::try_from(name_header_width.max(name_data_width) + 2)
                .unwrap_or(u16::MAX);
            let code_col_width = u16::try_from(code_header_width.max(code_data_width) + 2)
                .unwrap_or(u16::MAX);

            let widths = vec![
                Constraint::Length(name_col_width),
                Constraint::Length(code_col_width),
                Constraint::Length(3),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(14),
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
            format!(
                "No rows • Sort: {} {} • s next • d flip • Esc back",
                sort_state.field.label(),
                sort_state.direction_icon()
            )
        } else if chart_state.show {
            format!(
                "Row {}/{} • {}-{} of {} • Sort: {} {} • s next • d flip • ↑/↓ move • PgUp/PgDn page • Home/End jump • Enter/←/→/h/l timeframe • X close • Esc back",
                selected + 1,
                total,
                offset + 1,
                visible_end,
                total,
                sort_state.field.label(),
                sort_state.direction_icon()
            )
        } else {
            format!(
                "Row {}/{} • {}-{} of {} • Sort: {} {} • s next • d flip • ↑/↓ move • PgUp/PgDn page • Home/End jump • Enter chart • Esc back",
                selected + 1,
                total,
                offset + 1,
                visible_end,
                total,
                sort_state.field.label(),
                sort_state.direction_icon()
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
                                chart_state.prepare_history(&stock.stock_code, &stock.market);
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
                    KeyCode::Char('s') => {
                        let current_code = rows_data
                            .get(selected)
                            .map(|stock| stock.stock_code.clone());
                        sort_state.cycle_field();
                        rows_data = rebuild_sorted_rows(
                            database,
                            codes,
                            sort_state,
                            current_code,
                            &mut selected,
                            &mut offset,
                            capacity,
                            &mut chart_state,
                        );
                    }
                    KeyCode::Char('d') => {
                        let current_code = rows_data
                            .get(selected)
                            .map(|stock| stock.stock_code.clone());
                        sort_state.toggle_direction();
                        rows_data = rebuild_sorted_rows(
                            database,
                            codes,
                            sort_state,
                            current_code,
                            &mut selected,
                            &mut offset,
                            capacity,
                            &mut chart_state,
                        );
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if chart_state.show {
                            chart_state.next_timeframe();
                        }
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
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
                                    chart_state.prepare_history(&stock.stock_code, &stock.market);
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
                                    chart_state.prepare_history(&stock.stock_code, &stock.market);
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
                                            .prepare_history(&stock.stock_code, &stock.market);
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
                                            .prepare_history(&stock.stock_code, &stock.market);
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
                                    chart_state.prepare_history(&stock.stock_code, &stock.market);
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
                                    chart_state.prepare_history(&stock.stock_code, &stock.market);
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
