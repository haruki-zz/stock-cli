use std::collections::HashMap;
use std::path::Path;

use crate::error::{Context, Result};

use crate::config::Threshold;
use crate::fetch::StockData;
use crate::utils::current_human_timestamp;

/// Key/label pairs for all numeric metrics that can be filtered by thresholds.
pub const FILTERABLE_METRICS: &[(&str, &str)] = &[
    ("curr", "Last Price"),
    ("prevClosed", "Previous Close"),
    ("open", "Open"),
    ("increase", "Change (%)"),
    ("highest", "Intraday High"),
    ("lowest", "Intraday Low"),
    ("turnOver", "Turnover"),
    ("amp", "Amplitude"),
    ("tm", "TM"),
];

/// Ensure every expected metric has a threshold entry so downstream views remain in sync.
pub fn ensure_metric_thresholds(thresholds: &mut HashMap<String, Threshold>) {
    for (key, _) in FILTERABLE_METRICS {
        thresholds
            .entry((*key).to_string())
            .or_insert_with(|| Threshold {
                lower: 0.0,
                upper: 0.0,
                valid: false,
            });
    }
}

/// Minimal container for the in-memory stock snapshot plus persistence helpers.
pub struct StockDatabase {
    pub data: Vec<StockData>,
}

impl StockDatabase {
    pub fn new(data: Vec<StockData>) -> Self {
        Self { data }
    }

    /// Return the codes of stocks whose metrics fall within every active threshold.
    pub fn filter_stocks(&self, thresholds: &HashMap<String, Threshold>) -> Vec<String> {
        self.data
            .iter()
            .filter(|stock| {
                thresholds
                    .iter()
                    .filter(|(_, threshold)| threshold.valid)
                    .all(|(metric, threshold)| match metric_value(stock, metric) {
                        Some(value) => value >= threshold.lower && value <= threshold.upper,
                        None => true,
                    })
            })
            .map(|stock| stock.stock_code.clone())
            .collect()
    }

    pub fn update(&mut self, new_data: Vec<StockData>) {
        self.data = new_data;
        use std::io::{self, Write};
        let mut out = io::stdout();
        // Emit a timestamped note so the user sees when the buffer last refreshed.
        let _ = write!(
            out,
            "Stock information is updated on {}.\r\n",
            current_human_timestamp()
        );
        let _ = out.flush();
    }

    /// Persist the current snapshot to disk so it can be reloaded by the CLI later.
    pub fn save_to_csv<P: AsRef<Path>>(&self, file_path: P) -> Result<()> {
        let path = file_path.as_ref();
        let mut writer = csv::Writer::from_path(path).context("Failed to create CSV writer")?;

        writer.write_record(&[
            "market",
            "stockName",
            "stockCode",
            "curr",
            "prevClosed",
            "open",
            "increase",
            "highest",
            "lowest",
            "turnOver",
            "amp",
            "tm",
        ])?;

        for stock in &self.data {
            writer.write_record(&[
                &stock.market,
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

    /// Load a snapshot produced by `save_to_csv` back into memory.
    pub fn load_from_csv<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        let path = file_path.as_ref();
        let mut reader = csv::Reader::from_path(path).context("Failed to open CSV file")?;

        let mut data = Vec::new();

        for result in reader.records() {
            let record = result.context("Failed to read CSV record")?;

            let has_market_column = record.len() > 11;
            let market = if has_market_column {
                record.get(0).unwrap_or("CN").trim().to_string()
            } else {
                "CN".to_string()
            };
            let base_index = if has_market_column { 1 } else { 0 };

            let stock = StockData {
                market,
                stock_name: record.get(base_index).unwrap_or("").trim().to_string(),
                stock_code: record.get(base_index + 1).unwrap_or("").trim().to_string(),
                curr: record
                    .get(base_index + 2)
                    .unwrap_or("0")
                    .trim()
                    .parse()
                    .unwrap_or(0.0),
                prev_closed: record
                    .get(base_index + 3)
                    .unwrap_or("0")
                    .trim()
                    .parse()
                    .unwrap_or(0.0),
                open: record
                    .get(base_index + 4)
                    .unwrap_or("0")
                    .trim()
                    .parse()
                    .unwrap_or(0.0),
                increase: record
                    .get(base_index + 5)
                    .unwrap_or("0")
                    .trim()
                    .parse()
                    .unwrap_or(0.0),
                highest: record
                    .get(base_index + 6)
                    .unwrap_or("0")
                    .trim()
                    .parse()
                    .unwrap_or(0.0),
                lowest: record
                    .get(base_index + 7)
                    .unwrap_or("0")
                    .trim()
                    .parse()
                    .unwrap_or(0.0),
                turn_over: record
                    .get(base_index + 8)
                    .unwrap_or("0")
                    .trim()
                    .parse()
                    .unwrap_or(0.0),
                amp: record
                    .get(base_index + 9)
                    .unwrap_or("0")
                    .trim()
                    .parse()
                    .unwrap_or(0.0),
                tm: record
                    .get(base_index + 10)
                    .unwrap_or("0")
                    .trim()
                    .parse()
                    .unwrap_or(0.0),
            };

            data.push(stock);
        }

        Ok(Self::new(data))
    }
}

fn metric_value(stock: &StockData, metric: &str) -> Option<f64> {
    match metric {
        "curr" => Some(stock.curr),
        "prevClosed" => Some(stock.prev_closed),
        "open" => Some(stock.open),
        "increase" => Some(stock.increase),
        "highest" => Some(stock.highest),
        "lowest" => Some(stock.lowest),
        "turnOver" => Some(stock.turn_over),
        "amp" => Some(stock.amp),
        "tm" => Some(stock.tm),
        _ => None,
    }
}
