use anyhow::{Context, Result};
use chrono::{Local, LocalResult, NaiveDate, TimeZone};
use reqwest::{
    blocking::Client,
    header::{ACCEPT_LANGUAGE, REFERER, USER_AGENT},
};
use serde_json::Value;
use std::{
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

const HISTORY_ENDPOINT: &str = "https://ifzq.gtimg.cn/appstock/app/kline/kline";
const HISTORY_REFERER: &str = "https://gu.qq.com/";
const HISTORY_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36";
const HISTORY_ACCEPT_LANGUAGE: &str = "en-US,en;q=0.9";
const HISTORY_RECORD_DAYS: usize = 420;

#[derive(Clone)]
pub(crate) struct Candle {
    pub timestamp: chrono::DateTime<Local>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

pub(crate) type HistoryReceiver = Receiver<Result<Vec<Candle>>>;

pub(crate) fn spawn_history_fetch(stock_code: &str) -> HistoryReceiver {
    let code = stock_code.to_string();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = fetch_price_history(&code);
        let _ = tx.send(result);
    });

    rx
}

fn fetch_price_history(stock_code: &str) -> Result<Vec<Candle>> {
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
        anyhow::bail!("No historical data for {}", stock_code);
    }

    Ok(candles)
}
