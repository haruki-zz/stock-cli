use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use futures::stream::{self, StreamExt};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use crate::config::{InfoIndex, RegionConfig, Threshold};

#[derive(Debug, Clone)]
pub struct StockData {
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

pub struct AsyncStockFetcher {
    pub stock_list: Vec<String>,
    pub region_config: RegionConfig,
    pub info_indices: HashMap<String, InfoIndex>,
    pub client: Client,
    pub progress_counter: Arc<AtomicUsize>,
    pub total_stocks: usize,
}

impl AsyncStockFetcher {
    pub fn new(
        stock_list: Vec<String>,
        region_config: RegionConfig,
        info_indices: HashMap<String, InfoIndex>,
    ) -> Self {
        let total_stocks = stock_list.len();
        Self {
            stock_list,
            region_config,
            info_indices,
            client: Client::new(),
            progress_counter: Arc::new(AtomicUsize::new(0)),
            total_stocks,
        }
    }

    pub async fn fetch_data(&self) -> Result<Vec<StockData>> {
        let max_concurrent = 5;
        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));
        let progress_counter = Arc::clone(&self.progress_counter);
        
        progress_counter.store(0, Ordering::SeqCst);
        
        let results: Vec<Option<StockData>> = stream::iter(self.stock_list.iter())
            .map(|stock_code| {
                let semaphore = Arc::clone(&semaphore);
                let progress_counter = Arc::clone(&progress_counter);
                async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    let result = self.fetch_stock_data(stock_code).await;
                    
                    let current = progress_counter.fetch_add(1, Ordering::SeqCst) + 1;
                    let progress = (current as f64 / self.total_stocks as f64) * 100.0;
                    print!("\rProgress: {:.2}%", progress);
                    use std::io::{self, Write};
                    io::stdout().flush().unwrap();
                    
                    result.ok()
                }
            })
            .buffer_unordered(max_concurrent)
            .collect()
            .await;

        println!(); // New line after progress
        
        let valid_results: Vec<StockData> = results.into_iter().flatten().collect();
        
        if valid_results.is_empty() {
            anyhow::bail!("Failed to fetch any stock data");
        }
        
        Ok(valid_results)
    }

    async fn fetch_stock_data(&self, stock_code: &str) -> Result<StockData> {
        let url = format!(
            "{}{}{}",
            self.region_config.urls.request.prefix,
            stock_code,
            self.region_config.urls.request.suffix
        );

        let mut retry_count = 0;
        let max_retries = 3;

        while retry_count < max_retries {
            match self.client
                .get(&url)
                .headers({
                    let mut headers = reqwest::header::HeaderMap::new();
                    for (key, value) in &self.region_config.urls.request.headers {
                        if let (Ok(header_name), Ok(header_value)) = (
                            reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                            reqwest::header::HeaderValue::from_str(value)
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
                        anyhow::bail!("Request for stock {} was redirected", stock_code);
                    }
                    
                    if response.status() == 403 {
                        anyhow::bail!("Request for stock {} was blocked by firewall", stock_code);
                    }

                    if response.status().is_success() {
                        let text = response.text().await?;
                        
                        if text.contains(&self.region_config.urls.firewall_warning.text) {
                            anyhow::bail!("Request for stock {} was blocked by firewall", stock_code);
                        }

                        return self.parse_stock_data(stock_code, &text)
                            .context("Failed to parse stock data");
                    }
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= max_retries {
                        anyhow::bail!("Failed to fetch stock {} after {} retries: {}", 
                                     stock_code, max_retries, e);
                    }
                    sleep(Duration::from_millis(2_u64.pow(retry_count as u32) * 1000)).await;
                    continue;
                }
            }
        }

        anyhow::bail!("Failed to fetch stock data for {}", stock_code)
    }

    fn parse_stock_data(&self, stock_code: &str, text: &str) -> Result<StockData> {
        let json: Value = serde_json::from_str(text)
            .context("Failed to parse JSON response")?;

        let stock_data = json["data"][stock_code]["qt"][stock_code]
            .as_array()
            .context("Invalid stock data structure")?;

        let get_string_value = |key: &str| -> Result<String> {
            let idx = self.info_indices.get(key)
                .context(format!("Index for {} not found", key))?;
            
            stock_data.get(idx.index)
                .and_then(|v| v.as_str())
                .context(format!("Failed to get {} value", key))
                .map(|s| s.to_string())
        };

        let get_float_value = |key: &str| -> Result<f64> {
            let idx = self.info_indices.get(key)
                .context(format!("Index for {} not found", key))?;
            
            stock_data.get(idx.index)
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .context(format!("Failed to parse {} as float", key))
        };

        Ok(StockData {
            stock_name: get_string_value("stockName")?,
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
}

pub struct StockDatabase {
    pub data: Vec<StockData>,
}

impl StockDatabase {
    pub fn new(data: Vec<StockData>) -> Self {
        Self { data }
    }

    pub fn show_stock_info(&self, stock_codes: &[String]) {
        let filtered_data: Vec<&StockData> = self.data
            .iter()
            .filter(|stock| stock_codes.contains(&stock.stock_code))
            .collect();

        if filtered_data.is_empty() {
            println!("No matching stock found.");
            return;
        }

        self.display_table(&filtered_data);
    }

    pub fn filter_stocks(&self, thresholds: &HashMap<String, Threshold>) -> Vec<String> {
        self.data
            .iter()
            .filter(|stock| {
                thresholds.iter().all(|(metric, threshold)| {
                    let value = match metric.as_str() {
                        "amp" => stock.amp,
                        "turnOver" => stock.turn_over,
                        "tm" => stock.tm,
                        "increase" => stock.increase,
                        _ => return true, // Unknown metric, skip filter
                    };
                    value >= threshold.lower && value <= threshold.upper
                })
            })
            .map(|stock| stock.stock_code.clone())
            .collect()
    }

    pub fn update(&mut self, new_data: Vec<StockData>) {
        self.data = new_data;
        let now: DateTime<Local> = Local::now();
        println!("Stock information is updated on {}.", now.format("%Y-%m-%d %H:%M"));
    }

    fn display_table(&self, stocks: &[&StockData]) {
        use unicode_width::UnicodeWidthStr;

        let headers = vec![
            "Stock Name", "Stock Code", "Current", "Prev Closed", 
            "Open", "Increase", "Highest", "Lowest", "Turnover", "Amp", "TM"
        ];

        let rows: Vec<Vec<String>> = stocks.iter().map(|stock| {
            vec![
                stock.stock_name.clone(),
                stock.stock_code.clone(),
                format!("{:.2}", stock.curr),
                format!("{:.2}", stock.prev_closed),
                format!("{:.2}", stock.open),
                format!("{:.2}", stock.increase),
                format!("{:.2}", stock.highest),
                format!("{:.2}", stock.lowest),
                format!("{:.2}", stock.turn_over),
                format!("{:.2}", stock.amp),
                format!("{:.2}", stock.tm),
            ]
        }).collect();

        let all_rows: Vec<Vec<String>> = std::iter::once(headers.iter().map(|h| h.to_string()).collect())
            .chain(rows)
            .collect();

        let col_count = all_rows[0].len();
        let mut col_widths = vec![0; col_count];

        for row in &all_rows {
            for (i, cell) in row.iter().enumerate() {
                let width = cell.width();
                if width > col_widths[i] {
                    col_widths[i] = width;
                }
            }
        }

        let border = format!("+{}+", 
            col_widths.iter()
                .map(|w| "-".repeat(w + 2))
                .collect::<Vec<_>>()
                .join("+")
        );

        println!("{}", border);

        for (row_idx, row) in all_rows.iter().enumerate() {
            let formatted_row = row.iter()
                .zip(&col_widths)
                .map(|(cell, width)| {
                    let cell_width = cell.width();
                    let padding = width - cell_width;
                    format!(" {}{} ", " ".repeat(padding), cell)
                })
                .collect::<Vec<_>>()
                .join("|");
            
            println!("|{}|", formatted_row);
            
            if row_idx == 0 {
                println!("{}", border);
            }
        }

        println!("{}", border);
    }

    pub fn save_to_csv(&self, file_path: &str) -> Result<()> {
        let mut writer = csv::Writer::from_path(file_path)
            .context("Failed to create CSV writer")?;

        writer.write_record(&[
            "stockName", "stockCode", "curr", "prevClosed", "open", 
            "increase", "highest", "lowest", "turnOver", "amp", "tm"
        ])?;

        for stock in &self.data {
            writer.write_record(&[
                &stock.stock_name,
                &stock.stock_code,
                &stock.curr.to_string(),
                &stock.prev_closed.to_string(),
                &stock.open.to_string(),
                &stock.increase.to_string(),
                &stock.highest.to_string(),
                &stock.lowest.to_string(),
                &stock.turn_over.to_string(),
                &stock.amp.to_string(),
                &stock.tm.to_string(),
            ])?;
        }

        writer.flush()?;
        Ok(())
    }

    pub fn load_from_csv(file_path: &str) -> Result<Self> {
        let mut reader = csv::Reader::from_path(file_path)
            .context("Failed to open CSV file")?;

        let mut data = Vec::new();

        for result in reader.records() {
            let record = result.context("Failed to read CSV record")?;
            
            let stock = StockData {
                stock_name: record.get(0).unwrap_or("").to_string(),
                stock_code: record.get(1).unwrap_or("").to_string(),
                curr: record.get(2).unwrap_or("0").parse().unwrap_or(0.0),
                prev_closed: record.get(3).unwrap_or("0").parse().unwrap_or(0.0),
                open: record.get(4).unwrap_or("0").parse().unwrap_or(0.0),
                increase: record.get(5).unwrap_or("0").parse().unwrap_or(0.0),
                highest: record.get(6).unwrap_or("0").parse().unwrap_or(0.0),
                lowest: record.get(7).unwrap_or("0").parse().unwrap_or(0.0),
                turn_over: record.get(8).unwrap_or("0").parse().unwrap_or(0.0),
                amp: record.get(9).unwrap_or("0").parse().unwrap_or(0.0),
                tm: record.get(10).unwrap_or("0").parse().unwrap_or(0.0),
            };
            
            data.push(stock);
        }

        Ok(Self::new(data))
    }
}