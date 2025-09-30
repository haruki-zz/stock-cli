use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
pub struct UrlConfig {
    pub request: RequestConfig,
    pub firewall_warning: FirewallWarning,
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
pub enum ProviderConfig {
    Tencent(TencentProviderConfig),
    Stooq(StooqProviderConfig),
}

#[derive(Debug, Clone)]
pub struct TencentProviderConfig {
    pub info_idxs: HashMap<String, InfoIndex>,
    pub urls: UrlConfig,
}

#[derive(Debug, Clone)]
pub struct StooqProviderConfig {
    pub symbol_suffix: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub regions: HashMap<String, RegionConfig>,
}

impl Config {
    pub fn builtin() -> Self {
        let info_idxs = HashMap::from([
            ("stockName".to_string(), InfoIndex { index: 1 }),
            ("stockCode".to_string(), InfoIndex { index: 2 }),
            ("curr".to_string(), InfoIndex { index: 3 }),
            ("prevClosed".to_string(), InfoIndex { index: 4 }),
            ("open".to_string(), InfoIndex { index: 5 }),
            ("increase".to_string(), InfoIndex { index: 32 }),
            ("highest".to_string(), InfoIndex { index: 33 }),
            ("lowest".to_string(), InfoIndex { index: 34 }),
            ("turnOver".to_string(), InfoIndex { index: 38 }),
            ("amp".to_string(), InfoIndex { index: 43 }),
            ("tm".to_string(), InfoIndex { index: 44 }),
        ]);

        let thre = HashMap::from([
            (
                "amp".to_string(),
                Threshold {
                    lower: 3.0,
                    upper: 6.0,
                    valid: false,
                },
            ),
            (
                "turnOver".to_string(),
                Threshold {
                    lower: 5.0,
                    upper: 10.0,
                    valid: true,
                },
            ),
            (
                "tm".to_string(),
                Threshold {
                    lower: 50.0,
                    upper: 120.0,
                    valid: true,
                },
            ),
            (
                "increase".to_string(),
                Threshold {
                    lower: 3.0,
                    upper: 5.0,
                    valid: true,
                },
            ),
        ]);

        let headers = HashMap::from([
            (
                "User-Agent".to_string(),
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36".to_string(),
            ),
            (
                "Referer".to_string(),
                "http://ifzq.gtimg.cn/appstock/app/kline".to_string(),
            ),
            (
                "Accept-Language".to_string(),
                "en-US,en;q=0.9".to_string(),
            ),
        ]);

        let region_cn = RegionConfig {
            code: "CN".to_string(),
            name: "China A-Shares".to_string(),
            stock_code_file: "stock_code.csv".to_string(),
            thresholds: thre,
            provider: ProviderConfig::Tencent(TencentProviderConfig {
                info_idxs,
                urls: UrlConfig {
                    request: RequestConfig {
                        prefix: "http://ifzq.gtimg.cn/appstock/app/kline/mkline?param=".to_string(),
                        suffix: ",m1,,10".to_string(),
                        headers,
                    },
                    firewall_warning: FirewallWarning {
                        text: "window.location.href=\"https://waf.tencent.com/501page.html?u="
                            .to_string(),
                    },
                },
            }),
        };

        let jp_thresholds = HashMap::from([
            (
                "increase".to_string(),
                Threshold {
                    lower: -5.0,
                    upper: 5.0,
                    valid: false,
                },
            ),
            (
                "amp".to_string(),
                Threshold {
                    lower: 0.0,
                    upper: 10.0,
                    valid: false,
                },
            ),
            (
                "turnOver".to_string(),
                Threshold {
                    lower: 0.0,
                    upper: 0.0,
                    valid: false,
                },
            ),
            (
                "tm".to_string(),
                Threshold {
                    lower: 0.0,
                    upper: 0.0,
                    valid: false,
                },
            ),
        ]);

        let region_jp = RegionConfig {
            code: "JP".to_string(),
            name: "Japan Stocks".to_string(),
            stock_code_file: "stock_codes/japan.csv".to_string(),
            thresholds: jp_thresholds,
            provider: ProviderConfig::Stooq(StooqProviderConfig {
                symbol_suffix: ".jp".to_string(),
            }),
        };

        let regions = HashMap::from([
            (region_cn.code.clone(), region_cn),
            (region_jp.code.clone(), region_jp),
        ]);

        Config { regions }
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
