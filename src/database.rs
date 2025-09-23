use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use std::collections::HashMap;

use crate::config::Threshold;
use crate::fetcher::StockData;

pub struct StockDatabase {
    pub data: Vec<StockData>,
}

impl StockDatabase {
    pub fn new(data: Vec<StockData>) -> Self {
        Self { data }
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
        use std::io::{self, Write};
        let mut out = io::stdout();
        let _ = write!(
            out,
            "Stock information is updated on {}.\r\n",
            now.format("%Y-%m-%d %H:%M")
        );
        let _ = out.flush();
    }

    pub fn save_to_csv(&self, file_path: &str) -> Result<()> {
        let mut writer =
            csv::Writer::from_path(file_path).context("Failed to create CSV writer")?;

        writer.write_record(&[
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
        let mut reader = csv::Reader::from_path(file_path).context("Failed to open CSV file")?;

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
