use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod cn;
mod jp;

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
pub struct TencentHistoryConfig {
    pub endpoint: String,
    pub referer: String,
    pub user_agent: String,
    pub accept_language: String,
    pub record_days: usize,
}

#[derive(Debug, Clone)]
pub struct TencentProviderConfig {
    pub snapshot: SnapshotConfig,
    pub history: TencentHistoryConfig,
}

#[derive(Debug, Clone)]
pub struct StooqHistoryConfig {
    pub endpoint: String,
}

#[derive(Debug, Clone)]
pub struct StooqProviderConfig {
    pub symbol_suffix: String,
    pub snapshot: SnapshotConfig,
    pub history: StooqHistoryConfig,
}

#[derive(Debug, Clone)]
pub enum ProviderConfig {
    Tencent(TencentProviderConfig),
    Stooq(StooqProviderConfig),
}

impl ProviderConfig {
    pub fn snapshot(&self) -> &SnapshotConfig {
        match self {
            ProviderConfig::Tencent(cfg) => &cfg.snapshot,
            ProviderConfig::Stooq(cfg) => &cfg.snapshot,
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
}

#[derive(Debug, Clone)]
pub struct Config {
    pub regions: HashMap<String, RegionConfig>,
}

impl Config {
    pub fn builtin() -> Self {
        let cn = cn::region();
        let jp = jp::region();

        let regions = HashMap::from([(cn.code.clone(), cn), (jp.code.clone(), jp)]);

        Self { regions }
    }

    /// Retrieve the full region configuration, including disabled entries.
    pub fn get_region_config(&self, region_code: &str) -> Option<&RegionConfig> {
        self.regions.get(region_code)
    }

    pub fn available_regions(&self) -> Vec<&RegionConfig> {
        let mut regions: Vec<&RegionConfig> = self.regions.values().collect();
        regions.sort_by(|a, b| a.code.cmp(&b.code));
        regions
    }
}
