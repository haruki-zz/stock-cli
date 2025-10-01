use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::error::{AppError, Context, Result};

use crate::config::{RegionConfig, Threshold};
use crate::utils::snapshot_timestamp_slug;

pub mod presets;
pub mod stock_database;

pub use stock_database::{ensure_metric_thresholds, StockDatabase, FILTERABLE_METRICS};

/// Facade that keeps snapshot and preset persistence isolated from the rest of the app.
pub struct Records {
    snapshots_dir: PathBuf,
    presets_dir: PathBuf,
}

impl Records {
    pub fn for_region(region: &RegionConfig) -> Self {
        let lower = region.code.to_lowercase();
        let snapshots_dir = PathBuf::from(format!("assets/snapshots/{}", lower));
        let presets_dir = PathBuf::from(format!("assets/filters/{}", lower));
        Self::with_dirs(snapshots_dir, presets_dir)
    }

    pub fn with_dirs<S, P>(snapshots_dir: S, presets_dir: P) -> Self
    where
        S: Into<PathBuf>,
        P: Into<PathBuf>,
    {
        Self {
            snapshots_dir: snapshots_dir.into(),
            presets_dir: presets_dir.into(),
        }
    }

    pub fn snapshots_dir(&self) -> &Path {
        &self.snapshots_dir
    }

    pub fn presets_dir(&self) -> &Path {
        &self.presets_dir
    }

    /// Ensure the target directories exist before any persistence happens.
    pub fn prepare(&self) -> Result<()> {
        if let Some(parent) = self.snapshots_dir.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create parent directory {} for snapshots",
                    parent.display()
                )
            })?;
        }
        fs::create_dir_all(&self.snapshots_dir).with_context(|| {
            format!(
                "Failed to create snapshots directory {}",
                self.snapshots_dir.display()
            )
        })?;

        if let Some(parent) = self.presets_dir.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Failed to create parent directory {} for presets",
                    parent.display()
                )
            })?;
        }
        fs::create_dir_all(&self.presets_dir).with_context(|| {
            format!(
                "Failed to create presets directory {}",
                self.presets_dir.display()
            )
        })?;
        Ok(())
    }

    /// Clone region defaults and enforce the expected metric keys.
    pub fn initial_thresholds(&self, region: &RegionConfig) -> HashMap<String, Threshold> {
        let mut thresholds = region.thresholds.clone();
        ensure_metric_thresholds(&mut thresholds);
        thresholds
    }

    /// Locate the most recently modified CSV snapshot on disk, if any.
    pub fn latest_snapshot(&self) -> Result<Option<(PathBuf, String)>> {
        let entries = match fs::read_dir(&self.snapshots_dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => {
                return Err(AppError::message(format!(
                    "Failed to list snapshots directory {}: {}",
                    self.snapshots_dir.display(),
                    err
                )));
            }
        };

        let mut latest: Option<(std::time::SystemTime, PathBuf)> = None;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("csv") {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(meta) => meta,
                Err(_) => continue,
            };
            let modified = match metadata.modified() {
                Ok(ts) => ts,
                Err(_) => continue,
            };

            if latest
                .as_ref()
                .map(|(ts, _)| modified > *ts)
                .unwrap_or(true)
            {
                latest = Some((modified, path));
            }
        }

        if let Some((_, path)) = latest {
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            Ok(Some((path, name)))
        } else {
            Ok(None)
        }
    }

    pub fn load_snapshot<P: AsRef<Path>>(&self, path: P) -> Result<StockDatabase> {
        StockDatabase::load_from_csv(path)
    }

    /// Persist the in-memory snapshot using a timestamped filename.
    pub fn save_snapshot(&self, database: &StockDatabase) -> Result<PathBuf> {
        let filename = format!("{}_raw.csv", snapshot_timestamp_slug());
        let path = self.snapshots_dir.join(filename);
        database.save_to_csv(&path)?;
        Ok(path)
    }

    pub fn save_threshold_preset(
        &self,
        name: &str,
        thresholds: &HashMap<String, Threshold>,
    ) -> Result<PathBuf> {
        let mut normalized = thresholds.clone();
        ensure_metric_thresholds(&mut normalized);
        presets::save_thresholds(self.presets_dir(), name, &normalized)
    }

    pub fn load_threshold_preset<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<HashMap<String, Threshold>> {
        let mut map = presets::load_thresholds(path.as_ref())?;
        ensure_metric_thresholds(&mut map);
        Ok(map)
    }
}
