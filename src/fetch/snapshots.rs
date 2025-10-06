use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::config::{
    DelimitedResponseConfig, JsonPathSegment, RegionConfig, SnapshotConfig, SnapshotResponse,
};
use crate::error::{AppError, Context};
use futures::stream::{self, StreamExt};
use reqwest::{header::HeaderMap, Client, StatusCode};
use serde_json::Value;
use tokio::time::{sleep, Duration};

use crate::fetch::{ensure_concurrency_limit, FetchResult, SNAPSHOT_CONCURRENCY_LIMIT};

#[derive(Debug, Clone)]
/// Canonical representation of a single stock row returned by the remote endpoint.
pub struct StockData {
    pub market: String,
    pub stock_name: String,
    pub stock_code: String,
    pub curr: f64,
    pub prev_closed: f64,
    pub open: f64,
    pub increase: f64,
    pub highest: f64,
    pub lowest: f64,
    pub turn_over: f64,
    pub amp: f64,
    pub tm: f64,
}

/// Fetches stock snapshots concurrently while exposing a shared progress counter for the UI.
pub struct SnapshotFetcher {
    pub stock_list: Vec<String>,
    pub region_config: RegionConfig,
    pub static_names: Arc<HashMap<String, String>>,
    pub client: Client,
    pub progress_counter: Arc<AtomicUsize>,
    pub total_stocks: usize,
    concurrency_limit: usize,
}

impl SnapshotFetcher {
    pub fn new(
        stock_list: Vec<String>,
        region_config: RegionConfig,
        static_names: HashMap<String, String>,
    ) -> Self {
        Self::with_concurrency_limit(
            stock_list,
            region_config,
            static_names,
            SNAPSHOT_CONCURRENCY_LIMIT,
        )
    }

    pub fn with_concurrency_limit(
        stock_list: Vec<String>,
        region_config: RegionConfig,
        static_names: HashMap<String, String>,
        concurrency_limit: usize,
    ) -> Self {
        let total_stocks = stock_list.len();
        Self {
            stock_list,
            region_config,
            static_names: Arc::new(static_names),
            client: Client::new(),
            progress_counter: Arc::new(AtomicUsize::new(0)),
            total_stocks,
            concurrency_limit: ensure_concurrency_limit(concurrency_limit),
        }
    }

    pub async fn fetch_data(&self) -> FetchResult<Vec<StockData>> {
        let concurrency_limit = self.concurrency_limit;
        let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency_limit));
        let progress_counter = Arc::clone(&self.progress_counter);

        progress_counter.store(0, Ordering::SeqCst);

        let results: Vec<Option<StockData>> = stream::iter(self.stock_list.clone().into_iter())
            .map(|stock_code_owned| {
                let semaphore = Arc::clone(&semaphore);
                let progress_counter = Arc::clone(&progress_counter);
                let this = self;
                async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    let result = this.fetch_stock_data(&stock_code_owned).await;

                    let _current = progress_counter.fetch_add(1, Ordering::SeqCst) + 1;

                    result.ok()
                }
            })
            .buffer_unordered(concurrency_limit)
            .collect()
            .await;

        let valid_results: Vec<StockData> = results.into_iter().flatten().collect();

        if valid_results.is_empty() {
            return Err(AppError::message("Failed to fetch any stock data"));
        }

        Ok(valid_results)
    }

    fn snapshot_config(&self) -> &SnapshotConfig {
        self.region_config.provider.snapshot()
    }

    async fn fetch_stock_data(&self, stock_code: &str) -> FetchResult<StockData> {
        let snapshot_cfg = self.snapshot_config();
        let prepared = prepare_request(stock_code, &snapshot_cfg.request)?;
        let response_text = self.perform_request(&prepared, stock_code).await?;
        validate_firewall(&response_text, snapshot_cfg)?;
        let raw_values = parse_response(stock_code, &response_text, &snapshot_cfg.response)?;
        build_stock_data(
            stock_code,
            &self.region_config,
            snapshot_cfg,
            &raw_values,
            &self.static_names,
        )
    }

    async fn perform_request(
        &self,
        prepared: &PreparedRequest,
        stock_code: &str,
    ) -> FetchResult<String> {
        let mut retry_count = 0;
        let max_retries = 3;

        loop {
            let request = self
                .client
                .get(&prepared.url)
                .headers(prepared.headers.clone());
            match request.send().await {
                Ok(response) => {
                    if response.status().is_redirection() {
                        return Err(AppError::message(format!(
                            "Request for stock {} was redirected",
                            stock_code
                        )));
                    }

                    if response.status() == StatusCode::FORBIDDEN {
                        return Err(AppError::message(format!(
                            "Request for stock {} was blocked by firewall",
                            stock_code
                        )));
                    }

                    if response.status().is_success() {
                        return response
                            .text()
                            .await
                            .context("Failed to read response body")
                            .map_err(AppError::from);
                    }

                    retry_count += 1;
                    if retry_count >= max_retries {
                        return Err(AppError::message(format!(
                            "Request for stock {} failed with status {}",
                            stock_code,
                            response.status()
                        )));
                    }
                }
                Err(err) => {
                    retry_count += 1;
                    if retry_count >= max_retries {
                        return Err(AppError::message(format!(
                            "Failed to fetch stock {} after {} retries: {}",
                            stock_code, max_retries, err
                        )));
                    }
                }
            }

            let delay = Duration::from_millis(2_u64.pow(retry_count as u32) * 1000);
            sleep(delay).await;
        }
    }
}

struct PreparedRequest {
    url: String,
    headers: HeaderMap,
}

fn prepare_request(
    stock_code: &str,
    request: &crate::config::RequestConfig,
) -> FetchResult<PreparedRequest> {
    if matches!(request.method, crate::config::HttpMethod::Get) {
        let code = request.code_transform.apply(stock_code);
        let url = request.url_template.replace("{code}", &code);
        let headers = build_headers(&request.headers)?;
        Ok(PreparedRequest { url, headers })
    } else {
        Err(AppError::message("Unsupported HTTP method"))
    }
}

fn build_headers(headers: &HashMap<String, String>) -> FetchResult<HeaderMap> {
    let mut map = HeaderMap::new();
    for (key, value) in headers {
        let name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
            .with_context(|| format!("Invalid header name: {}", key))?;
        let header_value = reqwest::header::HeaderValue::from_str(value)
            .with_context(|| format!("Invalid header value for {}", key))?;
        map.insert(name, header_value);
    }
    Ok(map)
}

fn validate_firewall(response_text: &str, snapshot_cfg: &SnapshotConfig) -> FetchResult<()> {
    if let Some(warning) = &snapshot_cfg.firewall_warning {
        if response_text.contains(&warning.text) {
            return Err(AppError::message("Request was blocked by firewall"));
        }
    }
    Ok(())
}

fn parse_response(
    stock_code: &str,
    text: &str,
    response: &SnapshotResponse,
) -> FetchResult<Vec<String>> {
    match response {
        SnapshotResponse::Json(cfg) => parse_json_response(stock_code, text, cfg),
        SnapshotResponse::Delimited(cfg) => parse_delimited_response(text, cfg),
    }
}

fn parse_json_response(
    stock_code: &str,
    text: &str,
    cfg: &crate::config::JsonResponseConfig,
) -> FetchResult<Vec<String>> {
    let json: Value = serde_json::from_str(text).context("Failed to parse JSON response")?;
    let mut cursor = &json;

    for segment in &cfg.data_path {
        cursor = match segment {
            JsonPathSegment::Key(key) => cursor.get(key).ok_or_else(|| {
                AppError::message(format!("Missing key '{}' in snapshot payload", key))
            })?,
            JsonPathSegment::StockCode => cursor.get(stock_code).ok_or_else(|| {
                AppError::message(format!(
                    "Missing data for stock {} in snapshot payload",
                    stock_code
                ))
            })?,
        };
    }

    let array = cursor
        .as_array()
        .context("Snapshot payload is not an array of values")?;

    Ok(array
        .iter()
        .map(|value| match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => String::new(),
            other => other.to_string(),
        })
        .collect())
}

fn parse_delimited_response(text: &str, cfg: &DelimitedResponseConfig) -> FetchResult<Vec<String>> {
    let line = text
        .lines()
        .skip(cfg.skip_lines)
        .find(|line| !line.trim().is_empty())
        .context("No quote data returned")?;

    Ok(line
        .split(cfg.delimiter)
        .map(|field| field.trim().trim_matches('"').to_string())
        .collect())
}

fn build_stock_data(
    stock_code: &str,
    region_config: &RegionConfig,
    snapshot_cfg: &SnapshotConfig,
    values: &[String],
    static_names: &HashMap<String, String>,
) -> FetchResult<StockData> {
    let lookup_value = |key: &str| -> Option<String> {
        snapshot_cfg
            .info_idxs
            .get(key)
            .and_then(|idx| values.get(idx.index))
            .map(|value| value.trim().to_string())
    };

    let parse_float = |key: &str| -> FetchResult<Option<f64>> {
        match lookup_value(key) {
            Some(value) if !value.is_empty() => value
                .parse::<f64>()
                .with_context(|| format!("Failed to parse {} as float", key))
                .map(Some)
                .map_err(AppError::from),
            Some(_) => Ok(None),
            None => Ok(None),
        }
    };

    let curr = parse_float("curr")?
        .ok_or_else(|| AppError::message("Missing current price in snapshot payload"))?;
    let prev_closed = parse_float("prevClosed")?.unwrap_or(curr);
    let open = parse_float("open")?.unwrap_or(curr);
    let highest = parse_float("highest")?.unwrap_or(curr);
    let lowest = parse_float("lowest")?.unwrap_or(curr);

    let stock_name = lookup_value("stockName")
        .filter(|name| !name.is_empty())
        .or_else(|| static_names.get(stock_code).cloned())
        .unwrap_or_else(|| stock_code.to_string());

    let increase = match parse_float("increase")? {
        Some(value) => value,
        None => percentage_change(curr, prev_closed),
    };

    let amp = match parse_float("amp")? {
        Some(value) => value,
        None => amplitude(highest, lowest, prev_closed),
    };

    let volume = parse_float("volume")?;

    let turn_over = match parse_float("turnOver")? {
        Some(value) => value,
        None => volume.map(|v| v / 1_000_000.0).unwrap_or(0.0),
    };

    let tm = match parse_float("tm")? {
        Some(value) => value,
        None => volume.map(|v| (v * curr) / 1_000_000.0).unwrap_or(0.0),
    };

    Ok(StockData {
        market: region_config.code.clone(),
        stock_name,
        stock_code: stock_code.to_string(),
        curr,
        prev_closed,
        open,
        increase,
        highest,
        lowest,
        turn_over,
        amp,
        tm,
    })
}

fn percentage_change(curr: f64, prev: f64) -> f64 {
    if prev.abs() > f64::EPSILON {
        ((curr - prev) / prev) * 100.0
    } else {
        0.0
    }
}

fn amplitude(high: f64, low: f64, prev: f64) -> f64 {
    if prev.abs() > f64::EPSILON {
        ((high - low) / prev) * 100.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stooq_line() {
        let snapshot_cfg = SnapshotConfig {
            request: crate::config::RequestConfig {
                method: crate::config::HttpMethod::Get,
                url_template: String::new(),
                headers: HashMap::new(),
                code_transform: crate::config::CodeTransform::default(),
            },
            response: SnapshotResponse::Delimited(DelimitedResponseConfig {
                delimiter: ',',
                skip_lines: 1,
            }),
            info_idxs: HashMap::from([
                ("curr".into(), InfoIndex { index: 6 }),
                ("prevClosed".into(), InfoIndex { index: 7 }),
                ("open".into(), InfoIndex { index: 3 }),
                ("highest".into(), InfoIndex { index: 4 }),
                ("lowest".into(), InfoIndex { index: 5 }),
                ("volume".into(), InfoIndex { index: 8 }),
            ]),
            firewall_warning: None,
        };

        let region_config = RegionConfig {
            code: "JP".into(),
            name: "Test".into(),
            stock_code_file: String::new(),
            thresholds: HashMap::new(),
            provider: ProviderConfig::Stooq(StooqProviderConfig {
                symbol_suffix: String::new(),
                snapshot: snapshot_cfg.clone(),
                history: StooqHistoryConfig {
                    endpoint: String::new(),
                },
            }),
        };

        let text = "Symbol,Date,Time,Open,High,Low,Close,PrevClose,Volume\n7203.JP,2024-01-02,15:00,2300,2350,2280,2340,2290,1800000\n";
        let values = parse_delimited_response(text, match &snapshot_cfg.response {
            SnapshotResponse::Delimited(cfg) => cfg,
            _ => unreachable!(),
        })
        .unwrap();

        let data = build_stock_data(
            "7203",
            &region_config,
            &snapshot_cfg,
            &values,
            &HashMap::new(),
        )
        .unwrap();

        assert_eq!(data.curr, 2340.0);
        assert_eq!(data.prev_closed, 2290.0);
        assert!((data.increase - ((2340.0 - 2290.0) / 2290.0 * 100.0)).abs() < 1e-6);
        assert_eq!(data.turn_over, 1.8);
    }
}
