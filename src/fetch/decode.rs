use std::borrow::Cow;

use chrono::{NaiveDate, ParseResult};
use serde_json::Value;

use crate::config::{JsonHistoryRowFormat, JsonPathSegment};
use crate::error::AppError;

use super::FetchResult;

pub fn walk_json_path<'a>(
    root: &'a Value,
    path: &[JsonPathSegment],
    raw_code: &str,
    transformed_code: Option<&str>,
) -> FetchResult<&'a Value> {
    let mut cursor = root;
    for segment in path {
        cursor = match segment {
            JsonPathSegment::Key(key) => cursor.get(key).ok_or_else(|| {
                AppError::message(format!("Missing key `{key}` while navigating JSON path"))
            })?,
            JsonPathSegment::StockCode => cursor
                .get(raw_code)
                .or_else(|| transformed_code.and_then(|code| cursor.get(code)))
                .ok_or_else(|| {
                    AppError::message(format!(
                        "Missing entry for stock `{raw_code}` while navigating JSON path"
                    ))
                })?,
        };
    }
    Ok(cursor)
}

pub fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

pub fn split_row<'a>(row: &'a Value, format: &JsonHistoryRowFormat) -> Option<Vec<Cow<'a, str>>> {
    match format {
        JsonHistoryRowFormat::Array(_) => {
            let array = row.as_array()?;
            Some(
                array
                    .iter()
                    .map(|value| Cow::Owned(value_to_string(value)))
                    .collect(),
            )
        }
        JsonHistoryRowFormat::StringDelimited { delimiter, .. } => {
            let text = row.as_str()?;
            Some(
                text.split(*delimiter)
                    .map(|part| Cow::Borrowed(part.trim()))
                    .collect(),
            )
        }
    }
}

pub fn split_csv_line<'a>(line: &'a str, delimiter: char) -> Vec<Cow<'a, str>> {
    line.split(delimiter)
        .map(|field| Cow::Borrowed(field.trim().trim_matches('"')))
        .collect()
}

pub fn parse_f64(value: &str) -> Option<f64> {
    value.parse::<f64>().ok()
}

pub fn parse_date(value: &str, format: &str) -> ParseResult<NaiveDate> {
    NaiveDate::parse_from_str(value, format)
}
