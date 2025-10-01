use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::error::{AppError, Context};
use calamine::{Data, Reader, Xlsx};
use futures::stream::{self, StreamExt};
use reqwest::{Client, StatusCode};
use tokio::time::{sleep, Duration};

use crate::config::{ProviderConfig, RegionConfig, StooqProviderConfig, TencentProviderConfig};
use crate::fetch::{ensure_concurrency_limit, FetchResult, SNAPSHOT_CONCURRENCY_LIMIT};

const STOOQ_QUOTE_ENDPOINT: &str = "https://stooq.com/q/l/";
const JP_LISTING_URL: &str =
    "https://www.jpx.co.jp/english/markets/statistics-equities/misc/tvdivq0000001vg2-att/jyoujyou(updated)_e.xlsx";

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

        // Fan out the request list while honouring the concurrency guard to stay friendly to the API.
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

        // No direct stdout printing here; UI reads progress_counter instead

        let valid_results: Vec<StockData> = results.into_iter().flatten().collect();

        if valid_results.is_empty() {
            return Err(AppError::message("Failed to fetch any stock data"));
        }

        Ok(valid_results)
    }

    async fn fetch_stock_data(&self, stock_code: &str) -> FetchResult<StockData> {
        match &self.region_config.provider {
            ProviderConfig::Tencent(cfg) => self.fetch_stock_data_tencent(stock_code, cfg).await,
            ProviderConfig::Stooq(cfg) => self.fetch_stock_data_stooq(stock_code, cfg).await,
        }
    }

    async fn fetch_stock_data_tencent(
        &self,
        stock_code: &str,
        cfg: &TencentProviderConfig,
    ) -> FetchResult<StockData> {
        let url = format!(
            "{}{}{}",
            cfg.urls.request.prefix, stock_code, cfg.urls.request.suffix
        );

        let mut retry_count = 0;
        let max_retries = 3;

        while retry_count < max_retries {
            match self
                .client
                .get(&url)
                .headers({
                    let mut headers = reqwest::header::HeaderMap::new();
                    for (key, value) in &cfg.urls.request.headers {
                        if let (Ok(header_name), Ok(header_value)) = (
                            reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                            reqwest::header::HeaderValue::from_str(value),
                        ) {
                            headers.insert(header_name, header_value);
                        }
                    }
                    headers
                })
                .send()
                .await
            {
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
                        let text = response.text().await?;

                        if text.contains(&cfg.urls.firewall_warning.text) {
                            return Err(AppError::message(format!(
                                "Request for stock {} was blocked by firewall",
                                stock_code
                            )));
                        }

                        return Ok(self
                            .parse_tencent_stock_data(stock_code, &text, cfg)
                            .context("Failed to parse stock data")?);
                    } else {
                        retry_count += 1;
                        if retry_count >= max_retries {
                            return Err(AppError::message(format!(
                                "Request for stock {} failed with status {}",
                                stock_code,
                                response.status()
                            )));
                        }
                        // Increase wait time for subsequent attempts to avoid hammering the upstream service.
                        let delay = Duration::from_millis(2_u64.pow(retry_count as u32) * 1000);
                        sleep(delay).await;
                        continue;
                    }
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= max_retries {
                        return Err(AppError::message(format!(
                            "Failed to fetch stock {} after {} retries: {}",
                            stock_code, max_retries, e
                        )));
                    }
                    // Back off exponentially on transport errors before retrying.
                    let delay = Duration::from_millis(2_u64.pow(retry_count as u32) * 1000);
                    sleep(delay).await;
                    continue;
                }
            }
        }

        Err(AppError::message(format!(
            "Failed to fetch stock data for {}",
            stock_code
        )))
    }

    fn parse_tencent_stock_data(
        &self,
        stock_code: &str,
        text: &str,
        cfg: &TencentProviderConfig,
    ) -> FetchResult<StockData> {
        let json: serde_json::Value =
            serde_json::from_str(text).context("Failed to parse JSON response")?;

        let stock_data = json["data"][stock_code]["qt"][stock_code]
            .as_array()
            .context("Invalid stock data structure")?;

        // Lazy closures keep the index lookup and type conversion logic uniform across fields.
        let get_string_value = |key: &str| -> FetchResult<String> {
            let idx = cfg
                .info_idxs
                .get(key)
                .context(format!("Index for {} not found", key))?;

            let value = stock_data
                .get(idx.index)
                .and_then(|v| v.as_str())
                .context(format!("Failed to get {} value", key))?;

            Ok(value.to_string())
        };

        let get_float_value = |key: &str| -> FetchResult<f64> {
            let idx = cfg
                .info_idxs
                .get(key)
                .context(format!("Index for {} not found", key))?;

            let value = stock_data
                .get(idx.index)
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .context(format!("Failed to parse {} as float", key))?;

            Ok(value)
        };

        let stock_name = match get_string_value("stockName") {
            Ok(name) => name,
            Err(_) => self
                .static_names
                .get(stock_code)
                .cloned()
                .unwrap_or_else(|| stock_code.to_string()),
        };

        Ok(StockData {
            market: self.region_config.code.clone(),
            stock_name,
            stock_code: stock_code.to_string(),
            curr: get_float_value("curr")?,
            prev_closed: get_float_value("prevClosed")?,
            open: get_float_value("open")?,
            increase: get_float_value("increase")?,
            highest: get_float_value("highest")?,
            lowest: get_float_value("lowest")?,
            turn_over: get_float_value("turnOver")?,
            amp: get_float_value("amp")?,
            tm: get_float_value("tm")?,
        })
    }

    async fn fetch_stock_data_stooq(
        &self,
        stock_code: &str,
        cfg: &StooqProviderConfig,
    ) -> FetchResult<StockData> {
        let symbol = format!("{}{}", stock_code.to_lowercase(), cfg.symbol_suffix);
        let url = format!(
            "{endpoint}?s={symbol}&f=sd2t2ohlcpv&h=1&e=csv",
            endpoint = STOOQ_QUOTE_ENDPOINT,
            symbol = symbol
        );

        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            return Err(AppError::message(format!(
                "Request for stock {} failed with status {}",
                stock_code,
                response.status()
            )));
        }

        let body = response.text().await?;
        let mut lines = body.lines();
        let _header = lines.next();
        let Some(data_line) = lines.next() else {
            return Err(AppError::message(format!(
                "No quote data returned for {}",
                stock_code
            )));
        };

        let fields: Vec<&str> = data_line.split(',').collect();
        if fields.len() < 9 {
            return Err(AppError::message(format!(
                "Unexpected quote format for {}",
                stock_code
            )));
        }

        let open = parse_number(fields[3])?;
        let high = parse_number(fields[4])?;
        let low = parse_number(fields[5])?;
        let close = parse_number(fields[6])?;
        let prev_close = parse_number(fields[7])?;
        let volume = parse_number(fields[8])?;

        let increase = if prev_close.abs() > f64::EPSILON {
            ((close - prev_close) / prev_close) * 100.0
        } else {
            0.0
        };
        let amp = if prev_close.abs() > f64::EPSILON {
            ((high - low) / prev_close) * 100.0
        } else {
            0.0
        };

        let turn_over = volume / 1_000_000.0;
        let tm = (volume * close) / 1_000_000.0;

        let name = self
            .static_names
            .get(stock_code)
            .cloned()
            .unwrap_or_else(|| stock_code.to_string());

        Ok(StockData {
            market: self.region_config.code.clone(),
            stock_name: name,
            stock_code: stock_code.to_string(),
            curr: close,
            prev_closed: prev_close,
            open,
            increase,
            highest: high,
            lowest: low,
            turn_over,
            amp,
            tm,
        })
    }
}

fn parse_number(value: &str) -> FetchResult<f64> {
    value
        .trim()
        .parse::<f64>()
        .map_err(|_| AppError::message(format!("Failed to parse numeric value: {}", value)))
}

/// Download the latest JPX-listed securities and return (code, name) pairs.
pub async fn fetch_japan_stock_codes() -> FetchResult<Vec<(String, String)>> {
    let response = reqwest::get(JP_LISTING_URL)
        .await
        .context("Failed to request JPX listings")?;

    if !response.status().is_success() {
        return Err(AppError::message(format!(
            "JPX listing request failed with status {}",
            response.status()
        )));
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read JPX listing payload")?;

    let cursor = Cursor::new(bytes);
    let mut workbook = Xlsx::new(cursor).context("Failed to parse JPX listing workbook")?;
    let range = workbook
        .worksheet_range("Sheet1")
        .context("Sheet1 not found in JPX listing workbook")?;

    let mut seen = HashSet::new();
    let mut entries = Vec::new();

    for row in range.rows().skip(1) {
        let Some(code_cell) = row.get(1) else {
            continue;
        };
        let Some(name_cell) = row.get(2) else {
            continue;
        };

        let Some(code) = format_code(code_cell) else {
            continue;
        };
        let Some(name) = cell_to_string(name_cell) else {
            continue;
        };

        if seen.insert(code.clone()) {
            entries.push((code, name));
        }
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(entries)
}

fn cell_to_string(cell: &Data) -> Option<String> {
    match cell {
        Data::String(s) => Some(s.trim().to_string()),
        Data::Float(f) => Some(format_number(*f)),
        Data::Int(i) => Some(i.to_string()),
        Data::Bool(b) => Some(b.to_string()),
        Data::DateTime(value) => Some(value.to_string()),
        Data::DateTimeIso(s) => Some(s.clone()),
        Data::DurationIso(s) => Some(s.clone()),
        Data::Empty => None,
        Data::Error(_) => None,
    }
}

fn format_code(cell: &Data) -> Option<String> {
    match cell {
        Data::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Data::Float(f) => {
            if f.is_finite() {
                let value = *f as i64;
                Some(format_with_padding(value))
            } else {
                None
            }
        }
        Data::Int(i) => Some(format_with_padding(*i)),
        Data::DateTime(_)
        | Data::DateTimeIso(_)
        | Data::DurationIso(_)
        | Data::Bool(_)
        | Data::Error(_) => None,
        _ => None,
    }
}

fn format_with_padding(value: i64) -> String {
    if value >= 10000 {
        value.to_string()
    } else {
        format!("{:04}", value)
    }
}

fn format_number(value: f64) -> String {
    if (value.fract()).abs() < f64::EPSILON {
        format_with_padding(value as i64)
    } else {
        value.to_string()
    }
}
