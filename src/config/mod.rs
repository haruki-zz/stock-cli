use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct InfoIndex {
    pub index: usize,
    pub valid: bool,
}

#[derive(Debug, Clone)]
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
    pub info_idxs: HashMap<String, InfoIndex>,
    pub thre: HashMap<String, Threshold>,
    pub urls: UrlConfig,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub regions: HashMap<String, RegionConfig>,
}

impl Config {
    pub fn builtin() -> Self {
        let info_idxs = HashMap::from([
            (
                "stockName".to_string(),
                InfoIndex {
                    index: 1,
                    valid: true,
                },
            ),
            (
                "stockCode".to_string(),
                InfoIndex {
                    index: 2,
                    valid: true,
                },
            ),
            (
                "curr".to_string(),
                InfoIndex {
                    index: 3,
                    valid: true,
                },
            ),
            (
                "prevClosed".to_string(),
                InfoIndex {
                    index: 4,
                    valid: true,
                },
            ),
            (
                "open".to_string(),
                InfoIndex {
                    index: 5,
                    valid: true,
                },
            ),
            (
                "increase".to_string(),
                InfoIndex {
                    index: 32,
                    valid: true,
                },
            ),
            (
                "highest".to_string(),
                InfoIndex {
                    index: 33,
                    valid: true,
                },
            ),
            (
                "lowest".to_string(),
                InfoIndex {
                    index: 34,
                    valid: true,
                },
            ),
            (
                "turnOver".to_string(),
                InfoIndex {
                    index: 38,
                    valid: true,
                },
            ),
            (
                "amp".to_string(),
                InfoIndex {
                    index: 43,
                    valid: true,
                },
            ),
            (
                "tm".to_string(),
                InfoIndex {
                    index: 44,
                    valid: true,
                },
            ),
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

        let region = RegionConfig {
            info_idxs,
            thre,
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
        };

        let regions = HashMap::from([("CN".to_string(), region)]);

        Config { regions }
    }

    /// Retrieve the full region configuration, including disabled entries.
    pub fn get_region_config(&self, region_code: &str) -> Option<&RegionConfig> {
        self.regions.get(region_code)
    }

    /// Return only the `infoIdxs` entries that are marked `valid` for the region.
    pub fn get_valid_info_indices(&self, region_code: &str) -> Option<HashMap<String, InfoIndex>> {
        self.get_region_config(region_code).map(|config| {
            config
                .info_idxs
                .iter()
                .filter(|(_, info)| info.valid)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
    }
}
