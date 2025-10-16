use chrono::{Duration as ChronoDuration, Local};
use ratatui::prelude::Stylize;
use ratatui::text::Line as TextLine;
use ratatui::{
    prelude::*,
    symbols::Marker,
    widgets::{
        canvas::{Canvas, Line as CanvasLine, Rectangle},
        Block, Borders, Paragraph, Wrap,
    },
};
use std::{cmp::Ordering, collections::HashMap, sync::mpsc::TryRecvError};

use crate::config::RegionConfig;
use crate::fetch::{spawn_history_fetch, Candle, HistoryReceiver, StockData};
use crate::ui::components::utils::split_vertical;

const TIMEFRAMES: &[(&str, ChronoDuration)] = &[
    ("1Y", ChronoDuration::weeks(52)),
    ("6M", ChronoDuration::weeks(26)),
    ("3M", ChronoDuration::weeks(13)),
    ("1M", ChronoDuration::days(30)),
    ("1W", ChronoDuration::days(7)),
];

const BODY_EPSILON: f64 = 1e-4;
const DATE_LABEL_FMT: &str = "%Y-%m-%d";
const DATE_LABEL_FMT_SHORT: &str = "%m-%d";
const DATE_LABEL_FMT_MEDIUM: &str = "%Y-%m";

/// Tracks chart state and caches per-stock historical data.
#[derive(Default)]
pub struct ChartState {
    pub show: bool,
    pub timeframe_index: usize,
    active_key: Option<String>,
    history_cache: HashMap<String, Vec<Candle>>,
    pending_fetches: HashMap<String, HistoryReceiver>,
    last_error: Option<String>,
}

impl ChartState {
    pub fn prepare_history(&mut self, region: &RegionConfig, stock_code: &str) {
        let market = region.code.as_str();
        let key = cache_key(market, stock_code);
        if self.active_key.as_deref() != Some(key.as_str()) {
            self.active_key = Some(key.clone());
            self.last_error = None;
        }

        if self.history_cache.contains_key(key.as_str()) {
            return;
        }

        if let Some(outcome) = self
            .pending_fetches
            .get(key.as_str())
            .map(|rx| rx.try_recv())
        {
            match outcome {
                Ok(Ok(history)) => {
                    self.history_cache.insert(key.clone(), history);
                    self.pending_fetches.remove(key.as_str());
                    self.last_error = None;
                }
                Ok(Err(err)) => {
                    self.pending_fetches.remove(key.as_str());
                    self.last_error = Some(err.to_string());
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.pending_fetches.remove(key.as_str());
                    self.last_error = Some("History fetch task ended unexpectedly".to_string());
                }
            }
            return;
        }

        if self
            .last_error
            .as_ref()
            .filter(|_| self.active_key.as_deref() == Some(key.as_str()))
            .is_some()
        {
            return;
        }
        let rx = spawn_history_fetch(stock_code, region);
        self.pending_fetches.insert(key, rx);
        self.last_error = None;
    }

    pub fn history_for(&self, market: &str, stock_code: &str) -> Option<&Vec<Candle>> {
        let key = cache_key(market, stock_code);
        self.history_cache.get(&key)
    }

    pub fn hide(&mut self) {
        self.show = false;
        self.last_error = None;
        self.active_key = None;
    }

    pub fn clear_active(&mut self) {
        self.active_key = None;
        self.last_error = None;
    }

    pub fn next_timeframe(&mut self) {
        self.timeframe_index = (self.timeframe_index + 1) % TIMEFRAMES.len();
    }

    pub fn prev_timeframe(&mut self) {
        self.timeframe_index = (self.timeframe_index + TIMEFRAMES.len() - 1) % TIMEFRAMES.len();
    }

    pub fn last_error(&self, market: &str, stock_code: &str) -> Option<&str> {
        let key = cache_key(market, stock_code);
        if self.active_key.as_deref() == Some(key.as_str()) {
            self.last_error.as_deref()
        } else {
            None
        }
    }
}

fn cache_key(market: &str, stock_code: &str) -> String {
    format!("{}:{}", market, stock_code)
}

pub fn render_chart_panel(
    f: &mut Frame<'_>,
    area: Rect,
    footer_height: u16,
    chart: &ChartState,
    stock: Option<&StockData>,
) {
    let segments = split_vertical(
        area,
        &[
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(footer_height),
        ],
    );
    let chart_area = Rect {
        x: segments[0].x,
        y: segments[0].y,
        width: segments[0].width,
        height: segments[0].height + segments[1].height,
    };
    let help_area = segments[2];

    let mut help_text = String::new();

    if let Some(stock) = stock {
        if let Some(history) = chart.history_for(&stock.market, &stock.stock_code) {
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
                let draw_series = {
                    let compressed = compress_to_width(&filtered, chart_area.width);
                    if compressed.is_empty() {
                        filtered.clone()
                    } else {
                        compressed
                    }
                };
                let series_len = draw_series.len().max(1);
                let width_px = chart_area.width.max(1) as f64;
                let height_px = chart_area.height.max(1) as f64;

                let left_margin = 7.0;
                let right_margin = 1.0;
                let top_margin = 1.0;
                let bottom_margin = 1.0;

                let axis_x = left_margin;
                let available_width = (width_px - left_margin - right_margin).max(1.0);
                let x_scale = if series_len > 1 {
                    available_width / (series_len.saturating_sub(1) as f64)
                } else {
                    0.0
                };
                let base_width = if series_len > 1 {
                    x_scale
                } else {
                    available_width
                };
                let half_body = (base_width * 0.35).clamp(0.03, 0.3);
                let half_wick = half_body.min(0.2).max(0.03);
                let axis_x_end = axis_x + available_width;
                let axis_tick_length_x = 5.0;

                let axis_y = bottom_margin;
                let available_height = (height_px - bottom_margin - top_margin).max(1.0);
                let price_range = (y_max - y_min).max(0.01);
                let price_scale = available_height / price_range;
                let axis_y_top = axis_y + available_height;

                let x_bounds = [0.0, width_px];
                let y_bounds = [-1.0, height_px];

                let y_tick_gap = 0.2;
                let y_tick_start = axis_x - y_tick_gap;
                let price_label_x = y_tick_start - axis_tick_length_x - 1.2;
                let price_tick_values = compute_price_ticks(y_min, y_min + price_range, 9)
                    .into_iter()
                    .filter(|value| value.is_finite())
                    .map(|value| {
                        let rounded = ((value * 100.0).round()) / 100.0;
                        (value, format!("{:.2}", rounded))
                    })
                    .fold(Vec::new(), |mut acc: Vec<(f64, String)>, (value, label)| {
                        if !acc
                            .iter()
                            .any(|(existing, _)| (existing - value).abs() < 1e-6)
                        {
                            acc.push((value, label));
                        }
                        acc
                    });

                let date_tick_positions = compute_date_ticks(&draw_series, 7)
                    .into_iter()
                    .map(|(idx, label)| (axis_x + idx as f64 * x_scale, label))
                    .collect::<Vec<_>>();

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

                let price_tick_values = price_tick_values;
                let date_tick_positions = date_tick_positions;
                let candles = draw_series.clone();
                let canvas = Canvas::default()
                    .block(Block::default().borders(Borders::ALL).title(format!(
                        "{} — {} | {}  (←/→/h/l cycle • X close)",
                        stock.stock_code, label, timeframe_legend
                    )))
                    .marker(Marker::HalfBlock)
                    .x_bounds(x_bounds)
                    .y_bounds(y_bounds)
                    .paint(move |ctx| {
                        let axis_color = Color::DarkGray;
                        for (idx, candle) in candles.iter().enumerate() {
                            let x = axis_x + (idx as f64) * x_scale;
                            let low = axis_y + (candle.low - y_min) * price_scale;
                            let high = axis_y + (candle.high - y_min) * price_scale;
                            let open = axis_y + (candle.open - y_min) * price_scale;
                            let close = axis_y + (candle.close - y_min) * price_scale;
                            let color = if candle.close >= candle.open {
                                Color::Green
                            } else {
                                Color::Red
                            };

                            ctx.draw(&CanvasLine {
                                x1: x,
                                y1: low,
                                x2: x,
                                y2: high,
                                color,
                            });

                            let body_top = open.max(close);
                            let body_bottom = open.min(close);
                            if (body_top - body_bottom).abs() < BODY_EPSILON {
                                ctx.draw(&CanvasLine {
                                    x1: x - half_wick,
                                    y1: body_top,
                                    x2: x + half_wick,
                                    y2: body_top,
                                    color,
                                });
                            } else {
                                ctx.draw(&Rectangle {
                                    x: x - half_body,
                                    y: body_bottom,
                                    width: half_body * 2.0,
                                    height: body_top - body_bottom,
                                    color,
                                });
                            }
                        }

                        ctx.layer();
                        ctx.draw(&CanvasLine {
                            x1: axis_x,
                            y1: axis_y,
                            x2: axis_x_end,
                            y2: axis_y,
                            color: axis_color,
                        });
                        ctx.draw(&CanvasLine {
                            x1: axis_x,
                            y1: axis_y,
                            x2: axis_x,
                            y2: axis_y_top,
                            color: axis_color,
                        });

                        for (value, label) in price_tick_values.iter() {
                            let coord = axis_y + (value - y_min) * price_scale;
                            if coord < axis_y - 0.001 || coord > axis_y_top + 0.001 {
                                continue;
                            }
                            ctx.print(price_label_x, coord, label.clone());
                        }

                        for (x_pos, label) in date_tick_positions.iter() {
                            ctx.print(*x_pos, -1.0, label.clone());
                        }
                    });

                f.render_widget(canvas, chart_area);

                help_text = format!(
                    "{} • {} sessions • {} -> {} • High {:.2} on {} • Low {:.2} on {}",
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
        } else if let Some(message) = chart.last_error(&stock.market, &stock.stock_code) {
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
        Paragraph::new(TextLine::from(help_text).gray()).wrap(Wrap { trim: true }),
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

fn compute_price_ticks(min: f64, max: f64, desired: usize) -> Vec<f64> {
    let desired = desired.max(2);
    if !min.is_finite() || !max.is_finite() {
        return vec![0.0, 1.0];
    }

    let mut effective_min = min;
    let mut effective_max = max.max(effective_min + f64::EPSILON);

    if (effective_max - effective_min).abs() < 1e-6 {
        let span = if effective_min.abs() < 1.0 {
            1.0
        } else {
            effective_min.abs() * 0.05
        };
        effective_min -= span / 2.0;
        effective_max += span / 2.0;
    }

    let step = (effective_max - effective_min) / (desired as f64 - 1.0);
    (0..desired)
        .map(|i| effective_min + step * i as f64)
        .collect()
}

fn compute_date_ticks(candles: &[Candle], desired: usize) -> Vec<(usize, String)> {
    if candles.is_empty() {
        return Vec::new();
    }

    let last_index = candles.len() - 1;
    if last_index == 0 {
        return vec![(0, candles[0].timestamp.format(DATE_LABEL_FMT).to_string())];
    }

    let desired = desired.max(2).min(candles.len());
    let step = (last_index as f64) / (desired.saturating_sub(1) as f64);
    let mut indices: Vec<usize> = (0..desired)
        .map(|i| ((i as f64 * step).round() as usize).min(last_index))
        .collect();
    indices.push(0);
    indices.push(last_index);
    indices.sort_unstable();
    indices.dedup();

    let first_ts = candles.first().unwrap().timestamp;
    let last_ts = candles.last().unwrap().timestamp;
    let total_days = (last_ts.date_naive() - first_ts.date_naive())
        .num_days()
        .abs();
    let mid_format = if total_days > 365 {
        DATE_LABEL_FMT_MEDIUM
    } else {
        DATE_LABEL_FMT_SHORT
    };

    indices
        .into_iter()
        .map(|idx| {
            let ts = candles[idx].timestamp;
            let label = if idx == 0 || idx == last_index {
                ts.format(DATE_LABEL_FMT).to_string()
            } else {
                ts.format(mid_format).to_string()
            };
            (idx, label)
        })
        .collect()
}
