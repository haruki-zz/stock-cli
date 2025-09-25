use chrono::{Duration as ChronoDuration, Local};
use ratatui::{
    prelude::*,
    symbols::Marker,
    widgets::{
        canvas::{Canvas, Line as CanvasLine, Rectangle},
        Block, Borders, Paragraph, Wrap,
    },
};
use std::{cmp::Ordering, collections::HashMap, sync::mpsc::TryRecvError};

use crate::core::history::{spawn_history_fetch, Candle, HistoryReceiver};

const TIMEFRAMES: &[(&str, ChronoDuration)] = &[
    ("1Y", ChronoDuration::weeks(52)),
    ("6M", ChronoDuration::weeks(26)),
    ("3M", ChronoDuration::weeks(13)),
    ("1M", ChronoDuration::days(30)),
    ("1W", ChronoDuration::days(7)),
];

const BODY_EPSILON: f64 = 1e-4;

/// Tracks chart state and caches per-stock historical data.
#[derive(Default)]
pub struct ChartState {
    pub show: bool,
    pub timeframe_index: usize,
    pub active_code: Option<String>,
    history_cache: HashMap<String, Vec<Candle>>,
    pending_fetches: HashMap<String, HistoryReceiver>,
    last_error: Option<String>,
}

impl ChartState {
    pub fn prepare_history(&mut self, stock_code: &str) {
        if self.active_code.as_deref() != Some(stock_code) {
            self.active_code = Some(stock_code.to_string());
            self.last_error = None;
        }

        if self.history_cache.contains_key(stock_code) {
            return;
        }

        if let Some(outcome) = self.pending_fetches.get(stock_code).map(|rx| rx.try_recv()) {
            match outcome {
                Ok(Ok(history)) => {
                    self.history_cache.insert(stock_code.to_string(), history);
                    self.pending_fetches.remove(stock_code);
                    self.last_error = None;
                }
                Ok(Err(err)) => {
                    self.pending_fetches.remove(stock_code);
                    self.last_error = Some(err.to_string());
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.pending_fetches.remove(stock_code);
                    self.last_error = Some("History fetch task ended unexpectedly".to_string());
                }
            }
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

        let rx = spawn_history_fetch(stock_code);
        self.pending_fetches.insert(stock_code.to_string(), rx);
        self.last_error = None;
    }

    pub fn history_for(&self, stock_code: &str) -> Option<&Vec<Candle>> {
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
    footer_height: u16,
    chart: &ChartState,
    stock: Option<&crate::core::fetcher::StockData>,
) {
    let segments = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(footer_height),
        ])
        .split(area);
    let chart_area = Rect {
        x: segments[0].x,
        y: segments[0].y,
        width: segments[0].width,
        height: segments[0].height + segments[1].height,
    };
    let help_area = segments[2];

    let mut help_text = String::new();

    if let Some(stock) = stock {
        if let Some(history) = chart.history_for(&stock.stock_code) {
            let (label, duration) = TIMEFRAMES[chart.timeframe_index];
            let filtered = filter_history(history, duration);
            if filtered.is_empty() {
                f.render_widget(
                    Paragraph::new("No historical data returned for this stock.")
                        .alignment(Alignment::Center)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(format!("{} — {}", stock.stock_code, label)),
                        ),
                    chart_area,
                );
            } else {
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

                let first = filtered.first().unwrap();
                let last = filtered.last().unwrap();

                let mut y_min = f64::INFINITY;
                let mut y_max = f64::NEG_INFINITY;
                for candle in &filtered {
                    if candle.low < y_min {
                        y_min = candle.low;
                    }
                    if candle.high > y_max {
                        y_max = candle.high;
                    }
                }
                let padding = ((y_max - y_min) * 0.05).max(0.01);
                let y_bounds = [y_min - padding, y_max + padding];

                let draw_series = {
                    let compressed = compress_to_width(&filtered, chart_area.width);
                    if compressed.is_empty() {
                        filtered.clone()
                    } else {
                        compressed
                    }
                };
                let series_len = draw_series.len().max(1);
                let x_bounds = [-0.5, (series_len.saturating_sub(1) as f64) + 0.5];

                let mut highest = &filtered[0];
                let mut lowest = &filtered[0];
                for candle in &filtered[1..] {
                    if candle
                        .high
                        .partial_cmp(&highest.high)
                        .unwrap_or(Ordering::Equal)
                        == Ordering::Greater
                    {
                        highest = candle;
                    }
                    if candle
                        .low
                        .partial_cmp(&lowest.low)
                        .unwrap_or(Ordering::Equal)
                        == Ordering::Less
                    {
                        lowest = candle;
                    }
                }

                let candles = draw_series.clone();
                let canvas = Canvas::default()
                    .block(Block::default().borders(Borders::ALL).title(format!(
                        "{} — {} | {}  (Enter/←/→ cycle • X close)",
                        stock.stock_code, label, timeframe_legend
                    )))
                    .marker(Marker::HalfBlock)
                    .x_bounds(x_bounds)
                    .y_bounds(y_bounds)
                    .paint(move |ctx| {
                        for (idx, candle) in candles.iter().enumerate() {
                            let x = idx as f64;
                            let color = if candle.close >= candle.open {
                                Color::Green
                            } else {
                                Color::Red
                            };

                            ctx.draw(&CanvasLine {
                                x1: x,
                                y1: candle.low,
                                x2: x,
                                y2: candle.high,
                                color,
                            });

                            let body_top = candle.open.max(candle.close);
                            let body_bottom = candle.open.min(candle.close);
                            if (body_top - body_bottom).abs() < BODY_EPSILON {
                                ctx.draw(&CanvasLine {
                                    x1: x - 0.3,
                                    y1: body_top,
                                    x2: x + 0.3,
                                    y2: body_top,
                                    color,
                                });
                            } else {
                                ctx.draw(&Rectangle {
                                    x: x - 0.3,
                                    y: body_bottom,
                                    width: 0.6,
                                    height: body_top - body_bottom,
                                    color,
                                });
                            }
                        }
                    });

                f.render_widget(canvas, chart_area);

                help_text = format!(
                    "{} • {} sessions {}→{} • High {:.2} on {} • Low {:.2} on {}",
                    label,
                    filtered.len(),
                    first.timestamp.format("%Y-%m-%d"),
                    last.timestamp.format("%Y-%m-%d"),
                    highest.high,
                    highest.timestamp.format("%Y-%m-%d"),
                    lowest.low,
                    lowest.timestamp.format("%Y-%m-%d"),
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
            help_text = "Fetching historical prices…".to_string();
        }
    } else {
        f.render_widget(
            Paragraph::new("No stock selected")
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("Price chart")),
            chart_area,
        );
        help_text = "Select a stock and press Enter to show candlesticks.".to_string();
    }

    f.render_widget(
        Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .wrap(Wrap { trim: true }),
        help_area,
    );
}

fn filter_history(history: &[Candle], duration: ChronoDuration) -> Vec<Candle> {
    let cutoff = Local::now() - duration;
    let filtered: Vec<Candle> = history
        .iter()
        .filter(|candle| candle.timestamp >= cutoff)
        .cloned()
        .collect();

    if filtered.len() < 2 {
        history.to_vec()
    } else {
        filtered
    }
}

fn compress_to_width(candles: &[Candle], width: u16) -> Vec<Candle> {
    let max_points = usize::from(width.max(1)) * 2;
    if candles.len() <= max_points || max_points == 0 {
        return candles.to_vec();
    }

    let stride = (candles.len() + max_points - 1) / max_points;
    let mut reduced = Vec::with_capacity(max_points);

    for chunk in candles.chunks(stride) {
        if chunk.is_empty() {
            continue;
        }
        let mut aggregated = chunk[0].clone();
        aggregated.open = chunk.first().unwrap().open;
        aggregated.close = chunk.last().unwrap().close;
        aggregated.high = chunk
            .iter()
            .map(|c| c.high)
            .fold(f64::NEG_INFINITY, f64::max);
        aggregated.low = chunk.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
        aggregated.timestamp = chunk.last().unwrap().timestamp;
        reduced.push(aggregated);
    }

    if reduced.len() > max_points {
        reduced.truncate(max_points);
    }

    reduced
}
