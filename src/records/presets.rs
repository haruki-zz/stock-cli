use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{Context, Result};

use crate::config::Threshold;

/// Persist the provided threshold set under the given preset name within `dir`.
pub fn save_thresholds(
    dir: &Path,
    name: &str,
    thresholds: &HashMap<String, Threshold>,
) -> Result<PathBuf> {
    fs::create_dir_all(dir).context("Failed to create presets directory")?;

    let mut path = dir.to_path_buf();
    path.push(format!("{}.json", name));

    let json = serde_json::to_string_pretty(thresholds)
        .context("Failed to serialize thresholds for preset")?;

    let mut file = fs::File::create(&path)
        .with_context(|| format!("Failed to create preset file {:?}", path))?;
    file.write_all(json.as_bytes())
        .with_context(|| format!("Failed to write preset file {:?}", path))?;

    Ok(path)
}

/// Load a threshold preset from disk.
pub fn load_thresholds(path: &Path) -> Result<HashMap<String, Threshold>> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("Failed to read preset file {:?}", path))?;
    let map = serde_json::from_str(&data)
        .with_context(|| format!("Failed to parse preset file {:?}", path))?;
    Ok(map)
}
