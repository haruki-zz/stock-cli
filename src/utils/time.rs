use chrono::{DateTime, Local};
use std::time::SystemTime;

pub fn format_file_modified(time: SystemTime) -> String {
    DateTime::<Local>::from(time)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

pub fn current_human_timestamp() -> String {
    Local::now().format("%Y-%m-%d %H:%M").to_string()
}

pub fn snapshot_timestamp_slug() -> String {
    Local::now().format("%Y_%m_%d_%H_%M").to_string()
}
