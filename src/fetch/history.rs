use std::borrow::Cow;
use std::io::Cursor;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use crate::config::{
    HistoryFieldIndices, HistoryResponse, JsonHistoryResponse, JsonHistoryRowFormat, RegionConfig,
};
use crate::error::{AppError, Context};
use crate::fetch::decode::{parse_date, parse_f64, split_row, value_to_string, walk_json_path};
use crate::fetch::request::{prepare_request, PreparedRequest, RequestContext};
use crate::fetch::FetchResult;
use chrono::{Local, LocalResult, TimeZone};
use csv::ReaderBuilder;
use reqwest::blocking::Client;
use serde_json::Value;

#[derive(Clone)]
pub struct Candle {
    pub timestamp: chrono::DateTime<Local>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

pub type HistoryReceiver = Receiver<FetchResult<Vec<Candle>>>;

pub fn spawn_history_fetch(stock_code: &str, region: &RegionConfig) -> HistoryReceiver {
    let code = stock_code.to_string();
    let region_config = region.clone();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = fetch_history(&code, &region_config);
        let _ = tx.send(result);
    });

    rx
}

fn fetch_history(stock_code: &str, region: &RegionConfig) -> FetchResult<Vec<Candle>> {
    let history_cfg = region.provider.history();
    let transformed_code = history_cfg.request.code_transform.apply(stock_code);

    let mut extras: Vec<(&str, Cow<'_, str>)> = Vec::new();
    if let Some(limit) = history_cfg.limit {
        extras.push(("record_days", Cow::Owned(limit.to_string())));
    }

    let prepared = prepare_request(
        &history_cfg.request,
        RequestContext {
            stock_code,
            region_code: &region.code,
            extras: &extras,
        },
    )?;

    let body = execute_request(stock_code, &prepared)?;
    let mut candles = match &history_cfg.response {
        HistoryResponse::JsonRows(cfg) => {
            parse_history_json(stock_code, &transformed_code, &body, cfg)?
        }
        HistoryResponse::CsvRows(cfg) => parse_history_csv(&body, cfg)?,
    };

    if candles.is_empty() {
        return Err(AppError::message(format!(
            "No historical data for {}",
            stock_code
        )));
    }

    candles.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    if let Some(limit) = history_cfg.limit {
        if candles.len() > limit {
            candles = candles.into_iter().rev().take(limit).collect::<Vec<_>>();
            candles.reverse();
        }
    }

    Ok(candles)
}

fn execute_request(stock_code: &str, prepared: &PreparedRequest) -> FetchResult<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("Failed to construct history HTTP client")?;

    let response = client
        .get(&prepared.url)
        .headers(prepared.headers.clone())
        .send()
        .with_context(|| format!("History request failed for {}", stock_code))?
        .error_for_status()
        .with_context(|| format!("History request returned error status for {}", stock_code))?;

    response
        .text()
        .with_context(|| format!("Failed to read history body for {}", stock_code))
        .map_err(AppError::from)
}

fn parse_history_json(
    stock_code: &str,
    transformed_code: &str,
    body: &str,
    cfg: &JsonHistoryResponse,
) -> FetchResult<Vec<Candle>> {
    let json: Value = serde_json::from_str(body)
        .with_context(|| format!("Failed to parse history JSON for {}", stock_code))?;
    let node = walk_json_path(&json, &cfg.data_path, stock_code, Some(transformed_code))?;
    let rows = node
        .as_array()
        .ok_or_else(|| AppError::message("History payload was not an array of rows"))?;

    let indices = match &cfg.row_format {
        JsonHistoryRowFormat::Array(indices) => indices,
        JsonHistoryRowFormat::StringDelimited { indices, .. } => indices,
    };

    let mut candles = Vec::with_capacity(rows.len());
    for row in rows {
        let parts: Vec<Cow<'_, str>> = match &cfg.row_format {
            JsonHistoryRowFormat::Array(_) => row
                .as_array()
                .map(|array| array.iter().map(value_to_string).collect::<Vec<_>>())
                .map(|vec| vec.into_iter().map(|s| Cow::Owned(s)).collect()),
            JsonHistoryRowFormat::StringDelimited { .. } => {
                split_row(row, &cfg.row_format).map(|vec| {
                    vec.into_iter()
                        .map(|c| Cow::Owned(c.into_owned()))
                        .collect()
                })
            }
        }
        .unwrap_or_else(|| Vec::new());

        if parts.is_empty() {
            continue;
        }

        if let Some(candle) = candle_from_parts(&parts, indices, &cfg.date_format) {
            candles.push(candle);
        }
    }

    Ok(candles)
}

fn parse_history_csv(
    body: &str,
    cfg: &crate::config::CsvHistoryResponse,
) -> FetchResult<Vec<Candle>> {
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .delimiter(cfg.delimiter as u8)
        .from_reader(Cursor::new(body));

    let mut candles = Vec::new();

    for (idx, result) in reader.records().enumerate() {
        let record = result.context("Failed to read historical record")?;
        if idx < cfg.skip_lines {
            continue;
        }

        let parts: Vec<Cow<'_, str>> = record
            .iter()
            .map(|field| Cow::Owned(field.trim().to_string()))
            .collect();

        if let Some(candle) = candle_from_parts(&parts, &cfg.indices, &cfg.date_format) {
            candles.push(candle);
        }
    }

    Ok(candles)
}

fn candle_from_parts(
    parts: &[Cow<'_, str>],
    indices: &HistoryFieldIndices,
    date_format: &str,
) -> Option<Candle> {
    let date = parts.get(indices.date)?.as_ref().trim();
    let open = parse_f64(parts.get(indices.open)?.as_ref().trim())?;
    let high = parse_f64(parts.get(indices.high)?.as_ref().trim())?;
    let low = parse_f64(parts.get(indices.low)?.as_ref().trim())?;
    let close = parse_f64(parts.get(indices.close)?.as_ref().trim())?;

    build_candle(date, open, high, low, close, date_format)
}

fn build_candle(
    date_str: &str,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    format: &str,
) -> Option<Candle> {
    let date = parse_date(date_str, format).ok()?;
    let naive = date.and_hms_opt(0, 0, 0)?;
    let timestamp = match Local.from_local_datetime(&naive) {
        LocalResult::Single(dt) => dt,
        LocalResult::Ambiguous(first, _) => first,
        LocalResult::None => return None,
    };

    Some(Candle {
        timestamp,
        open,
        high,
        low,
        close,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        HistoryFieldIndices, JsonHistoryResponse, JsonHistoryRowFormat, JsonPathSegment,
    };

    #[test]
    fn parses_json_history_rows() {
        let cfg = JsonHistoryResponse {
            data_path: vec![
                JsonPathSegment::Key("data".to_string()),
                JsonPathSegment::StockCode,
                JsonPathSegment::Key("day".to_string()),
            ],
            row_format: JsonHistoryRowFormat::Array(HistoryFieldIndices {
                date: 0,
                open: 1,
                close: 2,
                high: 3,
                low: 4,
            }),
            date_format: "%Y-%m-%d".to_string(),
        };

        let body = r#"{
            "data": {
                "sh600000": {
                    "day": [
                        ["2024-01-02","10.0","10.5","10.8","9.9"]
                    ]
                }
            }
        }"#;

        let candles =
            parse_history_json("sh600000", "sh600000", body, &cfg).expect("parse history");
        assert_eq!(candles.len(), 1);
        assert!((candles[0].open - 10.0).abs() < f64::EPSILON);
        assert!((candles[0].close - 10.5).abs() < f64::EPSILON);
    }
}
