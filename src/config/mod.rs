use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub mod loader;
pub mod registry;
pub mod validator;

#[derive(Debug, Clone)]
pub struct InfoIndex {
    pub index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threshold {
    pub lower: f64,
    pub upper: f64,
    pub valid: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum HttpMethod {
    Get,
}

#[derive(Debug, Clone, Default)]
pub struct CodeTransform {
    pub lowercase: bool,
    pub uppercase: bool,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
}

impl CodeTransform {
    pub fn apply(&self, code: &str) -> String {
        let mut transformed = code.to_string();
        if self.lowercase {
            transformed = transformed.to_lowercase();
        } else if self.uppercase {
            transformed = transformed.to_uppercase();
        }

        if let Some(prefix) = &self.prefix {
            transformed = format!("{}{}", prefix, transformed);
        }

        if let Some(suffix) = &self.suffix {
            transformed = format!("{}{}", transformed, suffix);
        }

        transformed
    }
}

#[derive(Debug, Clone)]
pub struct RequestConfig {
    pub method: HttpMethod,
    pub url_template: String,
    pub headers: HashMap<String, String>,
    pub code_transform: CodeTransform,
}

#[derive(Debug, Clone)]
pub struct FirewallWarning {
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct SnapshotConfig {
    pub request: RequestConfig,
    pub response: SnapshotResponse,
    pub info_idxs: HashMap<String, InfoIndex>,
    pub firewall_warning: Option<FirewallWarning>,
}

#[derive(Debug, Clone)]
pub enum SnapshotResponse {
    Json(JsonResponseConfig),
    #[allow(dead_code)]
    Delimited(DelimitedResponseConfig),
}

#[derive(Debug, Clone)]
pub struct JsonResponseConfig {
    pub data_path: Vec<JsonPathSegment>,
}

#[derive(Debug, Clone)]
pub enum JsonPathSegment {
    Key(String),
    StockCode,
}

#[derive(Debug, Clone)]
pub struct DelimitedResponseConfig {
    pub delimiter: char,
    pub skip_lines: usize,
}

#[derive(Debug, Clone)]
pub struct HistoryConfig {
    pub request: RequestConfig,
    pub response: HistoryResponse,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum HistoryResponse {
    JsonRows(JsonHistoryResponse),
    #[allow(dead_code)]
    CsvRows(CsvHistoryResponse),
}

#[derive(Debug, Clone)]
pub struct JsonHistoryResponse {
    pub data_path: Vec<JsonPathSegment>,
    pub row_format: JsonHistoryRowFormat,
    pub date_format: String,
}

#[derive(Debug, Clone)]
pub enum JsonHistoryRowFormat {
    Array(HistoryFieldIndices),
    StringDelimited {
        delimiter: char,
        indices: HistoryFieldIndices,
    },
}

#[derive(Debug, Clone)]
pub struct CsvHistoryResponse {
    pub delimiter: char,
    pub skip_lines: usize,
    pub indices: HistoryFieldIndices,
    pub date_format: String,
}

#[derive(Debug, Clone)]
pub struct HistoryFieldIndices {
    pub date: usize,
    pub open: usize,
    pub high: usize,
    pub low: usize,
    pub close: usize,
}

#[derive(Debug, Clone)]
pub struct RegionStorage {
    pub snapshots_dir: PathBuf,
    pub filters_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct TencentProviderConfig {
    pub snapshot: SnapshotConfig,
    pub history: HistoryConfig,
}

#[derive(Debug, Clone)]
pub struct StooqProviderConfig {
    pub snapshot: SnapshotConfig,
    pub history: HistoryConfig,
}

#[derive(Debug, Clone)]
pub enum ProviderConfig {
    Tencent(TencentProviderConfig),
    #[allow(dead_code)]
    Stooq(StooqProviderConfig),
}

impl ProviderConfig {
    pub fn snapshot(&self) -> &SnapshotConfig {
        match self {
            ProviderConfig::Tencent(cfg) => &cfg.snapshot,
            ProviderConfig::Stooq(cfg) => &cfg.snapshot,
        }
    }

    pub fn history(&self) -> &HistoryConfig {
        match self {
            ProviderConfig::Tencent(cfg) => &cfg.history,
            ProviderConfig::Stooq(cfg) => &cfg.history,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RegionConfig {
    pub code: String,
    pub name: String,
    pub stock_code_file: String,
    pub thresholds: HashMap<String, Threshold>,
    pub provider: ProviderConfig,
    pub storage: RegionStorage,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub regions: HashMap<String, RegionConfig>,
}

#[allow(unused_imports)]
pub use loader::{load_region_descriptor, load_region_descriptors, RegionDescriptor};
#[allow(unused_imports)]
pub use registry::ConfigRegistry;
#[allow(unused_imports)]
pub use validator::{validate_region_descriptor, validate_region_descriptors};

impl Config {
    #[allow(dead_code)]
    pub fn builtin() -> Self {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let descriptors = load_region_descriptors(&root).unwrap_or_default();
        let regions = descriptors
            .iter()
            .map(|descriptor| {
                let region: RegionConfig = descriptor.into();
                (region.code.clone(), region)
            })
            .collect();
        Self { regions }
    }

    /// Retrieve the full region configuration, including disabled entries.
    #[allow(dead_code)]
    pub fn get_region_config(&self, region_code: &str) -> Option<&RegionConfig> {
        self.regions.get(region_code)
    }

    #[allow(dead_code)]
    pub fn available_regions(&self) -> Vec<&RegionConfig> {
        let mut regions: Vec<&RegionConfig> = self.regions.values().collect();
        regions.sort_by(|a, b| a.code.cmp(&b.code));
        regions
    }
}

impl From<&RegionDescriptor> for RegionConfig {
    fn from(descriptor: &RegionDescriptor) -> Self {
        RegionConfig {
            code: descriptor.code.clone(),
            name: descriptor.name.clone(),
            stock_code_file: descriptor.stock_list_file.to_string_lossy().to_string(),
            thresholds: descriptor.thresholds.clone(),
            provider: descriptor.provider.clone(),
            storage: descriptor.storage.clone(),
        }
    }
}
