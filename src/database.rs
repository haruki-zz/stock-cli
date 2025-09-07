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

    pub fn show_stock_info(&self, stock_codes: &[String]) {
        let filtered_data: Vec<&StockData> = self
            .data
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
        println!(
            "Stock information is updated on {}.",
            now.format("%Y-%m-%d %H:%M")
        );
    }

    fn display_table(&self, stocks: &[&StockData]) {
        use unicode_width::UnicodeWidthStr;

        let headers = vec![
            "Stock Name", "Stock Code", "Current", "Prev Closed", "Open", "Increase",
            "Highest", "Lowest", "Turnover", "Amp", "TM",
        ];

        let rows: Vec<Vec<String>> = stocks
            .iter()
            .map(|stock| {
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
            })
            .collect();

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

        let border = format!(
            "+{}+",
            col_widths
                .iter()
                .map(|w| "-".repeat(w + 2))
                .collect::<Vec<_>>()
                .join("+")
        );

        println!("{}", border);

        for (row_idx, row) in all_rows.iter().enumerate() {
            let formatted_row = row
                .iter()
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
        let mut writer = csv::Writer::from_path(file_path).context("Failed to create CSV writer")?;

        writer.write_record(&[
            "stockName", "stockCode", "curr", "prevClosed", "open", "increase", "highest",
            "lowest", "turnOver", "amp", "tm",
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

