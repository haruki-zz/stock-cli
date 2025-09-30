use anyhow::{Context, Result};
use calamine::{Data, Reader, Xlsx};
use std::collections::HashSet;
use std::io::Cursor;

const JP_LISTING_URL: &str =
    "https://www.jpx.co.jp/english/markets/statistics-equities/misc/tvdivq0000001vg2-att/jyoujyou(updated)_e.xlsx";

/// Download the latest JPX-listed securities and return (code, name) pairs.
pub async fn fetch_japan_stock_codes() -> Result<Vec<(String, String)>> {
    let response = reqwest::get(JP_LISTING_URL)
        .await
        .context("Failed to request JPX listings")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "JPX listing request failed with status {}",
            response.status()
        );
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
