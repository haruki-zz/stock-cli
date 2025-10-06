use crate::error::Result;

pub mod history;
pub mod snapshots;

pub use history::{spawn_history_fetch, Candle, HistoryReceiver};
pub use snapshots::{SnapshotFetcher, StockData};

/// Default concurrency guard applied when issuing snapshot requests.
pub const SNAPSHOT_CONCURRENCY_LIMIT: usize = 5;

pub type FetchResult<T> = Result<T>;

#[inline]
pub fn ensure_concurrency_limit(limit: usize) -> usize {
    limit.max(1)
}
