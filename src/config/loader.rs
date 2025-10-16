use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::error::{AppError, Context, Result};

use super::{
    CodeTransform, CsvHistoryResponse, DelimitedResponseConfig, FirewallWarning, HistoryConfig,
    HistoryFieldIndices, HistoryResponse, HttpMethod, InfoIndex, JsonHistoryResponse,
    JsonHistoryRowFormat, JsonPathSegment, JsonResponseConfig, ProviderConfig, RegionStorage,
    RequestConfig, SnapshotConfig, SnapshotResponse, StooqProviderConfig, TencentProviderConfig,
    Threshold,
};
use crate::config::validator;

/// Loaded market definition composed from a CSV stock list and JSON provider configuration.
#[derive(Debug, Clone)]
pub struct RegionDescriptor {
    pub code: String,
    pub name: String,
    #[allow(dead_code)]
    pub stock_list_file: PathBuf,
    pub stock_codes: Vec<String>,
    pub thresholds: HashMap<String, Threshold>,
    pub provider: ProviderConfig,
    pub storage: RegionStorage,
}

/// Load a region descriptor by combining the JSON market configuration with the stock list CSV.
pub fn load_region_descriptor(root: &Path, region_slug: &str) -> Result<RegionDescriptor> {
    let json_path = root
        .join("assets")
        .join("configs")
        .join(format!("{region_slug}.json"));

    let json = fs::read_to_string(&json_path).with_context(|| {
        format!(
            "failed to read region config JSON at {}",
            json_path.display()
        )
    })?;

    let raw: RawRegionConfig = serde_json::from_str(&json).with_context(|| {
        format!(
            "failed to parse region config JSON at {}",
            json_path.display()
        )
    })?;

    ensure_region_code(&raw.code, region_slug, &json_path)?;

    let stock_path = resolve_stock_path(root, &raw.stock_list)?;
    let stock_codes = load_stock_codes(&stock_path)?;

    let thresholds = raw
        .thresholds
        .into_iter()
        .map(|(key, threshold)| (key, threshold.into_threshold()))
        .collect();

    let provider = raw.provider.into_provider_config()?;
    let storage = raw
        .storage
        .into_storage(root, &region_slug.to_lowercase())?;

    let descriptor = RegionDescriptor {
        code: raw.code,
        name: raw.name,
        stock_list_file: stock_path,
        stock_codes,
        thresholds,
        provider,
        storage,
    };

    validator::validate_region_descriptor(&descriptor)?;

    Ok(descriptor)
}

/// Discover and load every region descriptor under `assets/configs`.
#[allow(dead_code)]
pub fn load_region_descriptors(root: &Path) -> Result<Vec<RegionDescriptor>> {
    let configs_dir = root.join("assets").join("configs");
    if !configs_dir.exists() {
        return Ok(Vec::new());
    }

    let mut descriptors = Vec::new();
    for entry in fs::read_dir(&configs_dir).with_context(|| {
        format!(
            "failed to read region config directory {}",
            configs_dir.display()
        )
    })? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let slug = match path.file_stem().and_then(|stem| stem.to_str()) {
            Some(slug) => slug.to_string(),
            None => continue,
        };
        descriptors.push(load_region_descriptor(root, &slug)?);
    }

    descriptors.sort_by(|a, b| a.code.cmp(&b.code));
    Ok(descriptors)
}

fn resolve_stock_path(root: &Path, stock_list: &RawStockList) -> Result<PathBuf> {
    if stock_list.file.trim().is_empty() {
        return Err(AppError::message(
            "stock_list.file must be provided in region config JSON",
        ));
    }

    let path = root.join(&stock_list.file);
    if path.exists() {
        Ok(path)
    } else {
        Err(AppError::message(format!(
            "stock list file not found: {}",
            path.display()
        )))
    }
}

fn load_stock_codes(path: &Path) -> Result<Vec<String>> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read stock list CSV at {}", path.display()))?;

    let codes = contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect::<Vec<_>>();

    Ok(codes)
}

fn ensure_region_code(actual: &str, expected_slug: &str, source: &Path) -> Result<()> {
    let normalised_actual = actual.to_lowercase();
    let normalised_expected = expected_slug.to_lowercase();
    if normalised_actual == normalised_expected {
        Ok(())
    } else {
        Err(AppError::message(format!(
            "region code mismatch in {}: expected `{}`, found `{}`",
            source.display(),
            normalised_expected,
            actual
        )))
    }
}

#[derive(Debug, Deserialize)]
struct RawRegionConfig {
    code: String,
    name: String,
    #[serde(default)]
    stock_list: RawStockList,
    #[serde(default)]
    thresholds: HashMap<String, RawThreshold>,
    provider: RawProviderConfig,
    #[serde(default)]
    storage: RawStorageConfig,
}

#[derive(Debug, Deserialize, Default)]
struct RawStockList {
    file: String,
}

#[derive(Debug, Deserialize)]
struct RawThreshold {
    lower: f64,
    upper: f64,
    #[serde(default)]
    enabled: bool,
}

impl RawThreshold {
    fn into_threshold(self) -> Threshold {
        Threshold {
            lower: self.lower,
            upper: self.upper,
            valid: self.enabled,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RawProviderConfig {
    Tencent {
        snapshot: RawSnapshotConfig,
        history: RawHistoryConfig,
    },
    Stooq {
        snapshot: RawSnapshotConfig,
        history: RawHistoryConfig,
    },
}

impl RawProviderConfig {
    fn into_provider_config(self) -> Result<ProviderConfig> {
        match self {
            RawProviderConfig::Tencent { snapshot, history } => {
                Ok(ProviderConfig::Tencent(TencentProviderConfig {
                    snapshot: snapshot.into_snapshot_config()?,
                    history: history.into_history_config()?,
                }))
            }
            RawProviderConfig::Stooq { snapshot, history } => {
                Ok(ProviderConfig::Stooq(StooqProviderConfig {
                    snapshot: snapshot.into_snapshot_config()?,
                    history: history.into_history_config()?,
                }))
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawSnapshotConfig {
    request: RawRequestConfig,
    response: RawSnapshotResponse,
    #[serde(default)]
    firewall_warning: Option<String>,
    #[serde(default)]
    info_indices: HashMap<String, usize>,
}

impl RawSnapshotConfig {
    fn into_snapshot_config(self) -> Result<SnapshotConfig> {
        let request = self.request.into_request()?;
        let response = self.response.into_response()?;
        let info_idxs = self
            .info_indices
            .into_iter()
            .map(|(key, index)| {
                let info_index = InfoIndex { index };
                (key, info_index)
            })
            .collect();

        let firewall_warning = self.firewall_warning.map(|text| FirewallWarning { text });

        Ok(SnapshotConfig {
            request,
            response,
            info_idxs,
            firewall_warning,
        })
    }
}

#[derive(Debug, Deserialize)]
struct RawHistoryConfig {
    request: RawRequestConfig,
    response: RawHistoryResponse,
    #[serde(default)]
    limit: Option<usize>,
}

impl RawHistoryConfig {
    fn into_history_config(self) -> Result<HistoryConfig> {
        Ok(HistoryConfig {
            request: self.request.into_request()?,
            response: self.response.into_history_response()?,
            limit: self.limit,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RawHistoryResponse {
    JsonRows {
        path: Vec<String>,
        date_format: String,
        columns: RawHistoryColumns,
        #[serde(default)]
        row: RawJsonHistoryRowFormat,
    },
    CsvRows {
        delimiter: String,
        skip_lines: usize,
        date_format: String,
        columns: RawHistoryColumns,
    },
}

impl RawHistoryResponse {
    fn into_history_response(self) -> Result<HistoryResponse> {
        match self {
            RawHistoryResponse::JsonRows {
                path,
                date_format,
                columns,
                row,
            } => {
                let segments = path
                    .into_iter()
                    .map(parse_json_path_segment)
                    .collect::<Result<Vec<_>>>()?;
                let indices = columns.into_indices();
                let row_format = row.into_row_format(indices)?;
                Ok(HistoryResponse::JsonRows(JsonHistoryResponse {
                    data_path: segments,
                    row_format,
                    date_format,
                }))
            }
            RawHistoryResponse::CsvRows {
                delimiter,
                skip_lines,
                date_format,
                columns,
            } => {
                let character = delimiter.chars().next().ok_or_else(|| {
                    AppError::message("history.response.csv delimiter must not be empty")
                })?;

                Ok(HistoryResponse::CsvRows(CsvHistoryResponse {
                    delimiter: character,
                    skip_lines,
                    indices: columns.into_indices(),
                    date_format,
                }))
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawHistoryColumns {
    date: usize,
    open: usize,
    high: usize,
    low: usize,
    close: usize,
}

impl RawHistoryColumns {
    fn into_indices(self) -> HistoryFieldIndices {
        HistoryFieldIndices {
            date: self.date,
            open: self.open,
            high: self.high,
            low: self.low,
            close: self.close,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct RawJsonHistoryRowFormat {
    delimiter: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RawStorageConfig {
    snapshots_dir: Option<String>,
    filters_dir: Option<String>,
}

impl RawStorageConfig {
    fn into_storage(self, root: &Path, slug: &str) -> Result<RegionStorage> {
        let snapshots = self
            .snapshots_dir
            .unwrap_or_else(|| format!("assets/snapshots/{slug}"));
        let filters = self
            .filters_dir
            .unwrap_or_else(|| format!("assets/filters/{slug}"));

        if snapshots.trim().is_empty() {
            return Err(AppError::message("storage.snapshots_dir must not be empty"));
        }

        if filters.trim().is_empty() {
            return Err(AppError::message("storage.filters_dir must not be empty"));
        }

        Ok(RegionStorage {
            snapshots_dir: normalize_path(root, snapshots),
            filters_dir: normalize_path(root, filters),
        })
    }
}

fn normalize_path(root: &Path, value: String) -> PathBuf {
    let path = PathBuf::from(&value);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

impl RawJsonHistoryRowFormat {
    fn into_row_format(self, indices: HistoryFieldIndices) -> Result<JsonHistoryRowFormat> {
        if let Some(delimiter) = self.delimiter {
            let ch = delimiter.chars().next().ok_or_else(|| {
                AppError::message("history.response.json_rows.row.delimiter must not be empty")
            })?;
            Ok(JsonHistoryRowFormat::StringDelimited {
                delimiter: ch,
                indices,
            })
        } else {
            Ok(JsonHistoryRowFormat::Array(indices))
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawRequestConfig {
    method: String,
    url_template: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    code_transform: RawCodeTransform,
}

impl RawRequestConfig {
    fn into_request(self) -> Result<RequestConfig> {
        let method = parse_method(&self.method)?;
        let code_transform = self.code_transform.into_code_transform()?;

        Ok(RequestConfig {
            method,
            url_template: self.url_template,
            headers: self.headers,
            code_transform,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawCodeTransform {
    Named(String),
    Detailed {
        #[serde(default)]
        lowercase: bool,
        #[serde(default)]
        uppercase: bool,
        #[serde(default)]
        prefix: Option<String>,
        #[serde(default)]
        suffix: Option<String>,
    },
}

impl Default for RawCodeTransform {
    fn default() -> Self {
        RawCodeTransform::Named("default".to_string())
    }
}

impl RawCodeTransform {
    fn into_code_transform(self) -> Result<CodeTransform> {
        match self {
            RawCodeTransform::Named(name) => {
                let preset = name.trim().to_lowercase();
                match preset.as_str() {
                    "default" => Ok(CodeTransform::default()),
                    "uppercase" => Ok(CodeTransform {
                        uppercase: true,
                        ..CodeTransform::default()
                    }),
                    "lowercase" => Ok(CodeTransform {
                        lowercase: true,
                        ..CodeTransform::default()
                    }),
                    other => Err(AppError::message(format!(
                        "unsupported code_transform preset `{other}`"
                    ))),
                }
            }
            RawCodeTransform::Detailed {
                lowercase,
                uppercase,
                prefix,
                suffix,
            } => {
                if lowercase && uppercase {
                    return Err(AppError::message(
                        "code transform cannot request both lowercase and uppercase",
                    ));
                }

                Ok(CodeTransform {
                    lowercase,
                    uppercase,
                    prefix,
                    suffix,
                })
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum RawSnapshotResponse {
    JsonPath {
        path: Vec<String>,
    },
    Delimited {
        delimiter: String,
        #[serde(default)]
        skip_lines: usize,
    },
}

impl RawSnapshotResponse {
    fn into_response(self) -> Result<SnapshotResponse> {
        match self {
            RawSnapshotResponse::JsonPath { path } => {
                let segments = path
                    .into_iter()
                    .map(parse_json_path_segment)
                    .collect::<Result<Vec<_>>>()?;

                Ok(SnapshotResponse::Json(JsonResponseConfig {
                    data_path: segments,
                }))
            }
            RawSnapshotResponse::Delimited {
                delimiter,
                skip_lines,
            } => {
                let character = delimiter.chars().next().ok_or_else(|| {
                    AppError::message("delimiter cannot be empty in delimited response")
                })?;

                Ok(SnapshotResponse::Delimited(DelimitedResponseConfig {
                    delimiter: character,
                    skip_lines,
                }))
            }
        }
    }
}

fn parse_json_path_segment(value: String) -> Result<JsonPathSegment> {
    if value == "{symbol}" {
        Ok(JsonPathSegment::StockCode)
    } else if value.is_empty() {
        Err(AppError::message(
            "json_path entries must not be empty strings",
        ))
    } else {
        Ok(JsonPathSegment::Key(value))
    }
}

fn parse_method(value: &str) -> Result<HttpMethod> {
    match value {
        "GET" | "get" => Ok(HttpMethod::Get),
        other => Err(AppError::message(format!(
            "unsupported HTTP method `{other}` in snapshot request"
        ))),
    }
}
