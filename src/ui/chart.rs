use anyhow::{Context, Result};
use chrono::{DateTime, Duration as ChronoDuration, Local, LocalResult, NaiveDate, TimeZone};
use ratatui::{prelude::*, widgets::*};
use std::collections::HashMap;

const TIMEFRAMES: &[(&str, ChronoDuration)] = &[
    ("1Y", ChronoDuration::weeks(52)),
    ("6M", ChronoDuration::weeks(26)),
    ("3M", ChronoDuration::weeks(13)),
    ("1M", ChronoDuration::days(30)),
    ("1W", ChronoDuration::days(7)),
];

/// Tracks chart state and caches per-stock historical data.
#[derive(Default)]
pub struct ChartState {
    pub show: bool,
    pub timeframe_index: usize,
    pub active_code: Option<String>,
    history_cache: HashMap<String, Vec<(DateTime<Local>, f64)>>,
    last_error: Option<String>,
}

impl ChartState {
    pub fn prepare_history(&mut self, raw_data_dir: &str, stock_code: &str) {
        if self.active_code.as_deref() != Some(stock_code) {
            self.active_code = Some(stock_code.to_string());
            self.last_error = None;
        }

        if self.history_cache.contains_key(stock_code) {
            return;
        }

        if self
            .last_error
            .as_ref()
            .filter(|_| self.active_code.as_deref() == Some(stock_code))
            .is_some()
        {
            return;
        }

        match load_price_history(raw_data_dir, stock_code) {
            Ok(history) => {
                self.history_cache.insert(stock_code.to_string(), history);
                self.last_error = None;
            }
            Err(err) => {
                self.last_error = Some(err.to_string());
            }
        }
    }

    pub fn history_for(&self, stock_code: &str) -> Option<&Vec<(DateTime<Local>, f64)>> {
        self.history_cache.get(stock_code)
    }

    pub fn hide(&mut self) {
        self.show = false;
        self.last_error = None;
        self.active_code = None;
    }

    pub fn clear_active(&mut self) {
        self.active_code = None;
        self.last_error = None;
    }

    pub fn next_timeframe(&mut self) {
        self.timeframe_index = (self.timeframe_index + 1) % TIMEFRAMES.len();
    }

    pub fn prev_timeframe(&mut self) {
        self.timeframe_index = (self.timeframe_index + TIMEFRAMES.len() - 1) % TIMEFRAMES.len();
    }

    pub fn last_error(&self, stock_code: &str) -> Option<&str> {
        if self.active_code.as_deref() == Some(stock_code) {
            self.last_error.as_deref()
        } else {
            None
        }
    }
}

pub fn render_chart_panel(
    f: &mut Frame<'_>,
    area: Rect,
    chart: &ChartState,
    stock: Option<&crate::fetcher::StockData>,
) {
    let segments = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);
    let chart_area = Rect {
        x: segments[0].x,
        y: segments[0].y,
        width: segments[0].width,
        height: segments[0].height + segments[1].height,
    };
    let help_area = segments[2];

    if let Some(stock) = stock {
        if let Some(history) = chart.history_for(&stock.stock_code) {
            let (label, duration) = TIMEFRAMES[chart.timeframe_index];
            let filtered = filter_history(history, duration);

            if filtered.len() >= 2 {
                let data: Vec<(f64, f64)> = filtered
                    .iter()
                    .map(|(ts, price)| (ts.timestamp() as f64, *price))
                    .collect();

                let x_min = data.first().map(|p| p.0).unwrap_or(0.0);
                let x_max = data.last().map(|p| p.0).unwrap_or(0.0);
                let y_min = filtered
                    .iter()
                    .map(|(_, value)| *value)
                    .fold(f64::INFINITY, f64::min);
                let y_max = filtered
                    .iter()
                    .map(|(_, value)| *value)
                    .fold(f64::NEG_INFINITY, f64::max);
                let padding = ((y_max - y_min) * 0.05).max(0.01);
                let y_bounds = [y_min - padding, y_max + padding];

                let x_labels = vec![
                    Span::raw(filtered.first().unwrap().0.format("%Y-%m-%d").to_string()),
                    Span::raw(filtered.last().unwrap().0.format("%Y-%m-%d").to_string()),
                ];
                let y_labels = vec![
                    Span::raw(format!("{:.2}", y_min)),
                    Span::raw(format!("{:.2}", y_max)),
                ];

                let timeframe_legend = TIMEFRAMES
                    .iter()
                    .enumerate()
                    .map(|(idx, (lbl, _))| {
                        if idx == chart.timeframe_index {
                            format!("[{}]", lbl)
                        } else {
                            lbl.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("  ");

                let dataset = Dataset::default()
                    .name(label)
                    .graph_type(GraphType::Line)
                    .style(Style::default().fg(Color::Green))
                    .data(&data);

                let chart_widget = Chart::new(vec![dataset])
                    .block(Block::default().borders(Borders::ALL).title(format!(
                        "{} — {} | {}  (Enter/←/→ cycle • X close) ",
                        stock.stock_code, label, timeframe_legend
                    )))
                    .x_axis(Axis::default().bounds([x_min, x_max]).labels(x_labels))
                    .y_axis(Axis::default().bounds(y_bounds).labels(y_labels));
                f.render_widget(chart_widget, chart_area);
            } else {
                f.render_widget(
                    Paragraph::new("Not enough historical points to render a trend.")
                        .alignment(Alignment::Center)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(format!("{} — {}", stock.stock_code, label)),
                        ),
                    chart_area,
                );
            }
        } else if let Some(message) = chart.last_error(&stock.stock_code) {
            f.render_widget(
                Paragraph::new(message).alignment(Alignment::Center).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("{} — error", stock.stock_code)),
                ),
                chart_area,
            );
        } else {
            f.render_widget(
                Paragraph::new("Loading historical prices…")
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!("{} — loading", stock.stock_code)),
                    ),
                chart_area,
            );
        }
    } else {
        f.render_widget(
            Paragraph::new("No stock selected")
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("Price chart")),
            chart_area,
        );
    }

    f.render_widget(Paragraph::new(""), help_area);
}

fn filter_history(
    history: &[(DateTime<Local>, f64)],
    duration: ChronoDuration,
) -> Vec<(DateTime<Local>, f64)> {
    let cutoff = Local::now() - duration;
    let mut filtered: Vec<(DateTime<Local>, f64)> = history
        .iter()
        .filter(|(ts, _)| *ts >= cutoff)
        .cloned()
        .collect();

    if filtered.len() < 2 {
        filtered = history.to_vec();
    }

    filtered
}

fn load_price_history(raw_data_dir: &str, stock_code: &str) -> Result<Vec<(DateTime<Local>, f64)>> {
    let mut points = Vec::new();
    let entries = std::fs::read_dir(raw_data_dir)
        .with_context(|| format!("Failed to read raw data directory: {}", raw_data_dir))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("csv") {
            continue;
        }

        let file_stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(stem) => stem.trim_end_matches("_raw"),
            None => continue,
        };

        let parts: Vec<_> = file_stem.split('_').collect();
        if parts.len() < 5 {
            continue;
        }

        let (Ok(year), Ok(month), Ok(day), Ok(hour), Ok(minute)) = (
            parts[0].parse::<i32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
            parts[3].parse::<u32>(),
            parts[4].parse::<u32>(),
        ) else {
            continue;
        };

        let date = match NaiveDate::from_ymd_opt(year, month, day) {
            Some(date) => date,
            None => continue,
        };
        let naive = match date.and_hms_opt(hour, minute, 0) {
            Some(dt) => dt,
            None => continue,
        };
        let timestamp = match Local.from_local_datetime(&naive) {
            LocalResult::Single(dt) => dt,
            LocalResult::Ambiguous(first, _) => first,
            LocalResult::None => continue,
        };

        let mut reader = csv::Reader::from_path(&path)
            .with_context(|| format!("Failed to open {}", path.display()))?;
        for record in reader.records() {
            let record =
                record.with_context(|| format!("Failed to read record in {}", path.display()))?;
            if record.get(1).map(|s| s == stock_code).unwrap_or(false) {
                if let Some(price_str) = record.get(2) {
                    if let Ok(price) = price_str.parse::<f64>() {
                        points.push((timestamp, price));
                    }
                }
                break;
            }
        }
    }

    points.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(points)
}
