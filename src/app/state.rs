use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::{RegionConfig, Threshold};
use crate::error::{AppError, Context, Result};
use crate::fetch::StockData;
use crate::records::{Records, StockDatabase};

struct LoadedStockCodes {
    codes: Vec<String>,
    names: HashMap<String, String>,
}

/// Region-scoped runtime data shared across UI flows and fetch tasks.
pub struct RegionState {
    config: RegionConfig,
    records: Records,
    stock_codes: Vec<String>,
    stock_names: HashMap<String, String>,
    thresholds: HashMap<String, Threshold>,
    database: StockDatabase,
    loaded_file: Option<String>,
}

impl RegionState {
    /// Prepare the region by loading codes, ensuring directories, and seeding thresholds.
    pub async fn new(config: RegionConfig) -> Result<Self> {
        let LoadedStockCodes { codes, names } = prepare_stock_codes(&config).await?;

        let records = Records::for_region(&config);
        records.prepare()?;

        let thresholds = records.initial_thresholds(&config);

        Ok(Self {
            config,
            records,
            stock_codes: codes,
            stock_names: names,
            thresholds,
            database: StockDatabase::new(Vec::new()),
            loaded_file: None,
        })
    }

    pub fn config(&self) -> &RegionConfig {
        &self.config
    }

    pub fn stock_codes(&self) -> &[String] {
        &self.stock_codes
    }

    pub fn stock_names(&self) -> &HashMap<String, String> {
        &self.stock_names
    }

    pub fn thresholds(&self) -> &HashMap<String, Threshold> {
        &self.thresholds
    }

    pub fn thresholds_mut(&mut self) -> &mut HashMap<String, Threshold> {
        &mut self.thresholds
    }

    pub fn set_thresholds(&mut self, thresholds: HashMap<String, Threshold>) {
        self.thresholds = thresholds;
    }

    pub fn database(&self) -> &StockDatabase {
        &self.database
    }

    pub fn replace_database(&mut self, database: StockDatabase) {
        self.database = database;
    }

    pub fn records(&self) -> &Records {
        &self.records
    }

    pub fn loaded_file(&self) -> Option<&str> {
        self.loaded_file.as_deref()
    }

    pub fn set_loaded_file(&mut self, name: Option<String>) {
        self.loaded_file = name;
    }

    /// Replace the in-memory snapshot and persist it to disk.
    pub fn apply_snapshot(&mut self, data: Vec<StockData>) -> Result<PathBuf> {
        self.database.update(data);
        let saved_path = self.records.save_snapshot(&self.database)?;
        let name = saved_path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| saved_path.to_string_lossy().to_string());
        self.loaded_file = Some(name);
        Ok(saved_path)
    }

    pub fn directories(&self) -> (String, String) {
        let snapshots = self.records.snapshots_dir().to_string_lossy().to_string();
        let presets = self.records.presets_dir().to_string_lossy().to_string();
        (snapshots, presets)
    }
}

async fn prepare_stock_codes(region_config: &RegionConfig) -> Result<LoadedStockCodes> {
    let path = Path::new(&region_config.stock_code_file);

    if !path.exists() {
        return Err(AppError::message(format!(
            "Stock codes file not found for region {}: {}",
            region_config.code,
            path.display()
        )));
    }

    load_stock_codes(path)
}

fn load_stock_codes(file_path: &Path) -> Result<LoadedStockCodes> {
    if !file_path.exists() {
        return Err(AppError::message(format!(
            "Stock codes file not found: {}",
            file_path.display()
        )));
    }

    let mut reader = csv::Reader::from_path(file_path)
        .with_context(|| format!("Failed to open stock codes file {}", file_path.display()))?;

    let mut codes = Vec::new();
    let mut names = HashMap::new();

    for result in reader.records() {
        let record = result.context("Failed to read CSV record")?;
        if let Some(code_raw) = record.get(0) {
            let code = code_raw.trim();
            if code.is_empty() {
                continue;
            }
            let code_string = code.to_string();
            if let Some(name_raw) = record.get(1) {
                let name = name_raw.trim();
                if !name.is_empty() {
                    names.insert(code_string.clone(), name.to_string());
                }
            }
            codes.push(code_string);
        }
    }

    if codes.is_empty() {
        return Err(AppError::message(format!(
            "Stock codes file is empty: {}",
            file_path.display()
        )));
    }

    Ok(LoadedStockCodes { codes, names })
}
