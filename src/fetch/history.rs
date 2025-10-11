use std::io::Cursor;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use crate::config::{
    Config, HistoryProviderKind, ProviderConfig, StooqProviderConfig, TencentHistoryConfig,
};
use crate::error::{AppError, Context};
use chrono::{Local, LocalResult, NaiveDate, TimeZone};
use reqwest::{
    blocking::Client,
    header::{ACCEPT, ACCEPT_LANGUAGE, AUTHORIZATION, REFERER, USER_AGENT},
};
use serde_json::Value;

use crate::fetch::{snapshots::expand_env_vars, FetchResult};

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
    let region_config = Config::builtin().get_region_config(&market_code).cloned();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = match region_config {
            Some(region) => match &region.provider {
                ProviderConfig::Tencent(provider) => {
                    fetch_price_history_tencent(&code, &provider.history)
                }
                ProviderConfig::Stooq(provider) => fetch_price_history_stooq(&code, provider),
            },
            None => Err(AppError::message(format!(
                "Unknown market: {}",
                market_code
            ))),
        };
        let _ = tx.send(result);
    });

    rx
}

fn fetch_price_history_tencent(
    stock_code: &str,
    cfg: &TencentHistoryConfig,
) -> FetchResult<Vec<Candle>> {
    match cfg.kind {
        HistoryProviderKind::Tencent => fetch_price_history_tencent_cn(stock_code, cfg),
        HistoryProviderKind::Jquants => fetch_price_history_jquants(stock_code, cfg),
    }
}

fn fetch_price_history_tencent_cn(
    stock_code: &str,
    cfg: &TencentHistoryConfig,
) -> FetchResult<Vec<Candle>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("Failed to construct history HTTP client")?;
    let url = format!(
        "{}?param={},day,,,{}",
        cfg.endpoint, stock_code, cfg.record_days
    );

    let response = client
        .get(&url)
        .header(USER_AGENT, cfg.user_agent.as_str())
        .header(REFERER, cfg.referer.as_str())
        .header(ACCEPT_LANGUAGE, cfg.accept_language.as_str())
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

fn fetch_price_history_jquants(
    stock_code: &str,
    cfg: &TencentHistoryConfig,
) -> FetchResult<Vec<Candle>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("Failed to construct history HTTP client")?;

    let mut request = client.get(&cfg.endpoint).query(&[("code", stock_code)]);

    if !cfg.user_agent.is_empty() {
        request = request.header(USER_AGENT, cfg.user_agent.as_str());
    }
    if !cfg.referer.is_empty() {
        request = request.header(REFERER, cfg.referer.as_str());
    }
    if !cfg.accept_language.is_empty() {
        request = request.header(ACCEPT_LANGUAGE, cfg.accept_language.as_str());
    }

    request = request.header(ACCEPT, "application/json");

    if let Some(template) = &cfg.auth_header {
        let header_value = expand_env_vars(template)?;
        request = request.header(AUTHORIZATION, header_value);
    }

    let response = request
        .send()
        .with_context(|| format!("History request failed for {}", stock_code))?
        .error_for_status()
        .with_context(|| format!("History request returned error status for {}", stock_code))?;

    let body = response
        .text()
        .with_context(|| format!("Failed to read history body for {}", stock_code))?;

    parse_jquants_history(&body, stock_code, cfg.record_days)
}

fn parse_jquants_history(
    body: &str,
    requested_code: &str,
    record_days: usize,
) -> FetchResult<Vec<Candle>> {
    let root: Value = serde_json::from_str(body)
        .with_context(|| format!("Failed to parse history JSON for {}", requested_code))?;

    const ARRAY_KEYS: &[&str] = &[
        "daily_quotes",
        "price_daily",
        "price_daily_quotes",
        "prices",
        "data",
        "items",
        "results",
        "quotes",
    ];

    let entries = if let Some(arr) = root.as_array() {
        arr
    } else {
        ARRAY_KEYS
            .iter()
            .find_map(|key| root.get(*key).and_then(Value::as_array))
            .ok_or_else(|| AppError::message("Daily quotes payload missing or invalid"))?
    };

    let requested_norm = normalize_code(requested_code);
    let mut candles = Vec::with_capacity(entries.len());

    for entry in entries {
        let Some(object) = entry.as_object() else {
            continue;
        };

        if let Some(code_value) =
            find_value(object, &["Code", "SymbolCode", "IssueCode", "StockCode"])
        {
            if !matches_stock_code_value(code_value, &requested_norm) {
                continue;
            }
        }

        let Some(date_value) =
            find_value(object, &["Date", "BusinessDay", "TradeDate", "RecordDate"])
                .and_then(Value::as_str)
        else {
            continue;
        };

        let timestamp = parse_jquants_date(date_value)?;

        let Some(open) = parse_number_field(object, &["Open", "OpeningPrice"])? else {
            continue;
        };
        let Some(high) = parse_number_field(object, &["High", "HighPrice"])? else {
            continue;
        };
        let Some(low) = parse_number_field(object, &["Low", "LowPrice"])? else {
            continue;
        };
        let Some(close) = parse_number_field(object, &["Close", "ClosingPrice", "ClosePrice"])?
        else {
            continue;
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
    if record_days > 0 && candles.len() > record_days {
        let excess = candles.len() - record_days;
        candles.drain(0..excess);
    }

    if candles.is_empty() {
        return Err(AppError::message(format!(
            "No historical data for {}",
            requested_code
        )));
    }

    Ok(candles)
}

fn find_value<'a>(
    object: &'a serde_json::Map<String, Value>,
    aliases: &[&str],
) -> Option<&'a Value> {
    for alias in aliases {
        let alias_norm = normalize_key(alias);
        if let Some((_, value)) = object
            .iter()
            .find(|(key, _)| normalize_key(key) == alias_norm)
        {
            return Some(value);
        }
    }

    None
}

fn parse_number_field(
    object: &serde_json::Map<String, Value>,
    aliases: &[&str],
) -> FetchResult<Option<f64>> {
    match find_value(object, aliases) {
        Some(value) => json_number_to_f64(value),
        None => Ok(None),
    }
}

fn json_number_to_f64(value: &Value) -> FetchResult<Option<f64>> {
    match value {
        Value::Number(num) => num
            .as_f64()
            .ok_or_else(|| AppError::message("Numeric value out of range"))
            .map(Some),
        Value::String(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            trimmed
                .parse::<f64>()
                .with_context(|| format!("Failed to parse '{}' as float", trimmed))
                .map(Some)
                .map_err(AppError::from)
        }
        Value::Null => Ok(None),
        _ => Err(AppError::message(
            "Unexpected non-numeric value in history payload",
        )),
    }
}

fn parse_jquants_date(date_str: &str) -> FetchResult<chrono::DateTime<Local>> {
    let trimmed = date_str.trim();
    let naive_date = if trimmed.contains('-') {
        NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
            .with_context(|| format!("Failed to parse date '{}'", trimmed))?
    } else if trimmed.len() == 8 {
        NaiveDate::parse_from_str(trimmed, "%Y%m%d")
            .with_context(|| format!("Failed to parse date '{}'", trimmed))?
    } else {
        return Err(AppError::message(format!(
            "Unsupported date format '{}' in history payload",
            trimmed
        )));
    };

    let Some(naive_dt) = naive_date.and_hms_opt(0, 0, 0) else {
        return Err(AppError::message("Unable to construct timestamp from date"));
    };

    match Local.from_local_datetime(&naive_dt) {
        LocalResult::Single(dt) => Ok(dt),
        LocalResult::Ambiguous(first, _) => Ok(first),
        LocalResult::None => Err(AppError::message(
            "Failed to resolve local timestamp for history entry",
        )),
    }
}

fn matches_stock_code_value(value: &Value, requested_norm: &str) -> bool {
    if requested_norm.is_empty() {
        return true;
    }

    let Some(candidate) = value_to_string(value) else {
        return true;
    };

    let normalized = normalize_code(&candidate);
    normalized == requested_norm || normalized.starts_with(requested_norm)
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.trim().to_string()),
        Value::Number(num) => Some(num.to_string()),
        _ => None,
    }
}

fn normalize_key(key: &str) -> String {
    key.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn normalize_code(code: &str) -> String {
    code.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_jquants_history_payload() {
        let sample = r#"{
            "daily_quotes": [
                {
                    "Code": "7203",
                    "Date": "2024-01-04",
                    "Open": 2300.0,
                    "High": 2350.0,
                    "Low": 2280.0,
                    "Close": 2340.0
                },
                {
                    "Code": "7203.T",
                    "Date": "2024-01-05",
                    "OpeningPrice": "2340",
                    "HighPrice": "2380",
                    "LowPrice": "2320",
                    "ClosePrice": "2375"
                },
                {
                    "Code": "6758",
                    "Date": "2024-01-05",
                    "Open": 13000,
                    "High": 13100,
                    "Low": 12950,
                    "Close": 13050
                }
            ]
        }"#;

        let candles = parse_jquants_history(sample, "7203", 420).unwrap();

        assert_eq!(candles.len(), 2);
        assert!((candles[0].open - 2300.0).abs() < 1e-6);
        assert!((candles[1].close - 2375.0).abs() < 1e-6);
        assert!(candles[0].timestamp < candles[1].timestamp);
    }

    #[test]
    fn trims_history_to_record_days() {
        let sample = r#"{
            "daily_quotes": [
                {"Code": "7203", "Date": "2024-01-04", "Open": 1, "High": 2, "Low": 0, "Close": 1.5},
                {"Code": "7203", "Date": "2024-01-05", "Open": 2, "High": 3, "Low": 1.5, "Close": 2.5}
            ]
        }"#;

        let candles = parse_jquants_history(sample, "7203", 1).unwrap();

        assert_eq!(candles.len(), 1);
        assert!((candles[0].open - 2.0).abs() < 1e-6);
    }
}

fn fetch_price_history_stooq(
    stock_code: &str,
    cfg: &StooqProviderConfig,
) -> FetchResult<Vec<Candle>> {
    let symbol = format!("{}{}", stock_code.to_lowercase(), cfg.symbol_suffix);
    let url = format!(
        "{endpoint}?s={symbol}&i=d&h=1&e=csv",
        endpoint = cfg.history.endpoint,
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
