use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InfoIndex {
    pub index: usize,
    pub valid: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Threshold {
    pub lower: f64,
    pub upper: f64,
    pub valid: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RequestConfig {
    pub prefix: String,
    pub suffix: String,
    pub headers: HashMap<String, String>,
    pub valid: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FirewallWarning {
    pub text: String,
    pub valid: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UrlConfig {
    pub request: RequestConfig,
    #[serde(rename = "firewallWarning")]
    pub firewall_warning: FirewallWarning,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RegionConfig {
    #[serde(rename = "infoIdxs")]
    pub info_idxs: HashMap<String, InfoIndex>,
    pub thre: HashMap<String, Threshold>,
    pub urls: UrlConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(flatten)]
    pub regions: HashMap<String, RegionConfig>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        // Try multiple locations for the config file
        let search_paths = vec![
            path.to_path_buf(),
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|p| p.join(path)))
                .unwrap_or_else(|| path.to_path_buf()),
            std::env::current_dir()
                .map(|cwd| cwd.join(path))
                .unwrap_or_else(|_| path.to_path_buf()),
        ];

        let mut last_error = None;

        for search_path in search_paths {
            match std::fs::read_to_string(&search_path) {
                Ok(content) => {
                    let config: Config =
                        serde_json::from_str(&content).context("Failed to parse config JSON")?;
                    return Ok(config);
                }
                Err(e) => {
                    last_error = Some(format!("Failed to read {}: {}", search_path.display(), e));
                }
            }
        }

        anyhow::bail!(
            "Could not find config file. Last error: {}",
            last_error.unwrap_or_else(|| "No search paths".to_string())
        )
    }

    pub fn get_region_config(&self, region_code: &str) -> Option<&RegionConfig> {
        self.regions.get(region_code)
    }

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

    pub fn get_valid_thresholds(&self, region_code: &str) -> Option<HashMap<String, Threshold>> {
        self.get_region_config(region_code).map(|config| {
            config
                .thre
                .iter()
                .filter(|(_, threshold)| threshold.valid)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
    }
}
