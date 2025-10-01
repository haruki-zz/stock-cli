use std::io::Cursor;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use crate::error::{AppError, Context};
use chrono::{Local, LocalResult, NaiveDate, TimeZone};
use reqwest::{
    blocking::Client,
    header::{ACCEPT_LANGUAGE, REFERER, USER_AGENT},
};
use serde_json::Value;

use crate::fetch::FetchResult;

const HISTORY_ENDPOINT: &str = "https://ifzq.gtimg.cn/appstock/app/kline/kline";
const HISTORY_REFERER: &str = "https://gu.qq.com/";
const HISTORY_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36";
const HISTORY_ACCEPT_LANGUAGE: &str = "en-US,en;q=0.9";
const HISTORY_RECORD_DAYS: usize = 420;
const STOOQ_HISTORY_ENDPOINT: &str = "https://stooq.com/q/d/l/";
const STOOQ_SYMBOL_SUFFIX_JP: &str = ".jp";

#[derive(Clone)]
pub struct Candle {
    pub timestamp: chrono::DateTime<Local>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

pub type HistoryReceiver = Receiver<FetchResult<Vec<Candle>>>;

pub fn spawn_history_fetch(stock_code: &str, market: &str) -> HistoryReceiver {
    let code = stock_code.to_string();
    let market_code = market.to_string();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = match market_code.as_str() {
            "JP" => fetch_price_history_stooq(&code),
            _ => fetch_price_history_tencent(&code),
        };
        let _ = tx.send(result);
    });

    rx
}

fn fetch_price_history_tencent(stock_code: &str) -> FetchResult<Vec<Candle>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("Failed to construct history HTTP client")?;
    let url = format!(
        "{}?param={},day,,,{}",
        HISTORY_ENDPOINT, stock_code, HISTORY_RECORD_DAYS
    );

    let response = client
        .get(&url)
        .header(USER_AGENT, HISTORY_USER_AGENT)
        .header(REFERER, HISTORY_REFERER)
        .header(ACCEPT_LANGUAGE, HISTORY_ACCEPT_LANGUAGE)
        .send()
        .with_context(|| format!("History request failed for {}", stock_code))?
        .error_for_status()
        .with_context(|| format!("History request returned error status for {}", stock_code))?;

    let body = response
        .text()
        .with_context(|| format!("Failed to read history body for {}", stock_code))?;

    let root: Value = serde_json::from_str(&body)
        .with_context(|| format!("Failed to parse history JSON for {}", stock_code))?;

    let day_entries = root["data"][stock_code]["day"]
        .as_array()
        .context("Daily history payload missing or invalid")?;

    let mut candles = Vec::with_capacity(day_entries.len());
    for entry in day_entries {
        let row = match entry.as_array() {
            Some(row) => row,
            None => continue,
        };

        if row.len() < 5 {
            continue;
        }

        let Some(date_str) = row.get(0).and_then(Value::as_str) else {
            continue;
        };

        let parse_number = |value: &Value| -> Option<f64> {
            value
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .or_else(|| value.as_f64())
        };

        let Some(open) = row.get(1).and_then(parse_number) else {
            continue;
        };
        let Some(close) = row.get(2).and_then(parse_number) else {
            continue;
        };
        let Some(high) = row.get(3).and_then(parse_number) else {
            continue;
        };
        let Some(low) = row.get(4).and_then(parse_number) else {
            continue;
        };

        let date = match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(date) => date,
            Err(_) => continue,
        };
        let Some(naive) = date.and_hms_opt(0, 0, 0) else {
            continue;
        };
        let timestamp = match Local.from_local_datetime(&naive) {
            LocalResult::Single(dt) => dt,
            LocalResult::Ambiguous(first, _) => first,
            LocalResult::None => continue,
        };

        candles.push(Candle {
            timestamp,
            open,
            high,
            low,
            close,
        });
    }

    candles.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    if candles.is_empty() {
        return Err(AppError::message(format!(
            "No historical data for {}",
            stock_code
        )));
    }

    Ok(candles)
}

fn fetch_price_history_stooq(stock_code: &str) -> FetchResult<Vec<Candle>> {
    let symbol = format!("{}{}", stock_code.to_lowercase(), STOOQ_SYMBOL_SUFFIX_JP);
    let url = format!(
        "{endpoint}?s={symbol}&i=d&h=1&e=csv",
        endpoint = STOOQ_HISTORY_ENDPOINT,
        symbol = symbol
    );

    let response = reqwest::blocking::get(&url)
        .with_context(|| format!("History request failed for {}", stock_code))?;

    if !response.status().is_success() {
        return Err(AppError::message(format!(
            "History request returned error status {} for {}",
            response.status(),
            stock_code
        )));
    }

    let body = response
        .text()
        .with_context(|| format!("Failed to read history body for {}", stock_code))?;

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(Cursor::new(body));

    let mut candles = Vec::new();

    for result in reader.records() {
        let record = result.context("Failed to read historical record")?;
        let date_str = match record.get(0) {
            Some(value) if !value.is_empty() => value,
            _ => continue,
        };

        let parse_number = |idx: usize| -> Option<f64> {
            record
                .get(idx)
                .and_then(|field| field.trim().parse::<f64>().ok())
        };

        let open = match parse_number(1) {
            Some(value) => value,
            None => continue,
        };
        let high = match parse_number(2) {
            Some(value) => value,
            None => continue,
        };
        let low = match parse_number(3) {
            Some(value) => value,
            None => continue,
        };
        let close = match parse_number(4) {
            Some(value) => value,
            None => continue,
        };

        let date = match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(date) => date,
            Err(_) => continue,
        };
        let Some(naive) = date.and_hms_opt(0, 0, 0) else {
            continue;
        };
        let timestamp = match Local.from_local_datetime(&naive) {
            LocalResult::Single(dt) => dt,
            LocalResult::Ambiguous(first, _) => first,
            LocalResult::None => continue,
        };

        candles.push(Candle {
            timestamp,
            open,
            high,
            low,
            close,
        });
    }

    candles.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    if candles.is_empty() {
        return Err(AppError::message(format!(
            "No historical data for {}",
            stock_code
        )));
    }

    Ok(candles)
}
