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

#[derive(Debug, Clone)]
pub struct RequestConfig {
    pub prefix: String,
    pub suffix: String,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct FirewallWarning {
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct TencentSnapshotConfig {
    pub request: RequestConfig,
    pub firewall_warning: FirewallWarning,
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
    pub info_idxs: HashMap<String, InfoIndex>,
    pub snapshot: TencentSnapshotConfig,
    pub history: TencentHistoryConfig,
}

#[derive(Debug, Clone)]
pub struct StooqSnapshotConfig {
    pub quote_endpoint: String,
}

#[derive(Debug, Clone)]
pub struct StooqHistoryConfig {
    pub endpoint: String,
}

#[derive(Debug, Clone)]
pub struct StooqProviderConfig {
    pub symbol_suffix: String,
    pub snapshot: StooqSnapshotConfig,
    pub history: StooqHistoryConfig,
    pub listings_url: String,
}

#[derive(Debug, Clone)]
pub enum ProviderConfig {
    Tencent(TencentProviderConfig),
    Stooq(StooqProviderConfig),
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
