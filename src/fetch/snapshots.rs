use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::config::{DelimitedResponseConfig, RegionConfig, SnapshotConfig, SnapshotResponse};
use crate::error::{AppError, Context};
use futures::stream::{self, StreamExt};
use reqwest::{Client, StatusCode};
use serde_json::Value;
use tokio::time::{sleep, Duration};

use crate::fetch::decode::{split_csv_line, value_to_string, walk_json_path};
use crate::fetch::request::{prepare_request, PreparedRequest, RequestContext};
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
        let prepared = prepare_request(
            &snapshot_cfg.request,
            RequestContext {
                stock_code,
                region_code: &self.region_config.code,
                extras: &[],
            },
        )?;
        let response_text = self.perform_request(&prepared, stock_code).await?;
        validate_firewall(&response_text, snapshot_cfg)?;
        let values = parse_response(stock_code, &response_text, &snapshot_cfg.response)?;
        build_stock_data(
            stock_code,
            &self.region_config,
            snapshot_cfg,
            &values,
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
    let node = walk_json_path(&json, &cfg.data_path, stock_code, None)?;
    let array = node
        .as_array()
        .ok_or_else(|| AppError::message("Snapshot payload was not an array of values"))?;
    Ok(array.iter().map(value_to_string).collect())
}

fn parse_delimited_response(text: &str, cfg: &DelimitedResponseConfig) -> FetchResult<Vec<String>> {
    let line = text
        .lines()
        .skip(cfg.skip_lines)
        .find(|line| !line.trim().is_empty())
        .context("No quote data returned")?;

    Ok(split_csv_line(line, cfg.delimiter)
        .into_iter()
        .map(|field| field.into_owned())
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
