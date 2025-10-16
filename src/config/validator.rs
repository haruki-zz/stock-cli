use std::collections::HashMap;

use crate::error::{AppError, Result};

use super::{
    loader::RegionDescriptor, InfoIndex, JsonPathSegment, JsonResponseConfig, ProviderConfig,
    RequestConfig, SnapshotConfig, SnapshotResponse, Threshold,
};

/// Validate a single region descriptor and surface descriptive errors.
pub fn validate_region_descriptor(descriptor: &RegionDescriptor) -> Result<()> {
    let mut issues = Vec::new();

    validate_stock_list(descriptor, &mut issues);
    validate_thresholds(descriptor, &mut issues);
    validate_provider(descriptor, &mut issues);

    if issues.is_empty() {
        Ok(())
    } else {
        Err(AppError::message(format!(
            "region `{}` config invalid:\n  - {}",
            descriptor.code,
            issues.join("\n  - ")
        )))
    }
}

/// Validate a list of descriptors, aggregating per-region results.
#[allow(dead_code)]
pub fn validate_region_descriptors(descriptors: &[RegionDescriptor]) -> Result<()> {
    let mut issues = Vec::new();

    for descriptor in descriptors {
        if let Err(err) = validate_region_descriptor(descriptor) {
            issues.push(err.to_string());
        }
    }

    if issues.is_empty() {
        Ok(())
    } else {
        Err(AppError::message(format!(
            "invalid region configurations:\n{}",
            issues
                .into_iter()
                .map(|issue| format!("  - {issue}"))
                .collect::<Vec<_>>()
                .join("\n")
        )))
    }
}

fn validate_stock_list(descriptor: &RegionDescriptor, issues: &mut Vec<String>) {
    if descriptor.stock_codes.is_empty() {
        issues.push("stock list CSV yielded no symbols".to_string());
    }
}

fn validate_thresholds(descriptor: &RegionDescriptor, issues: &mut Vec<String>) {
    for (metric, threshold) in &descriptor.thresholds {
        if threshold.lower > threshold.upper {
            issues.push(format!(
                "threshold `{metric}` has lower bound {} greater than upper bound {}",
                threshold.lower, threshold.upper
            ));
        }
    }
}

fn validate_provider(descriptor: &RegionDescriptor, issues: &mut Vec<String>) {
    match &descriptor.provider {
        ProviderConfig::Tencent(provider) => {
            validate_snapshot_config(&provider.snapshot, issues);
            validate_tencent_history_config(&provider.history.endpoint, issues);
        }
        ProviderConfig::Stooq(provider) => {
            validate_snapshot_config(&provider.snapshot, issues);
            if provider.history.endpoint.trim().is_empty() {
                issues.push("provider.history.endpoint must not be empty".to_string());
            }
        }
    }
}

fn validate_snapshot_config(snapshot: &SnapshotConfig, issues: &mut Vec<String>) {
    validate_request(&snapshot.request, issues);
    validate_snapshot_response(&snapshot.response, issues);
    validate_info_indices(&snapshot.info_idxs, issues);
}

fn validate_request(request: &RequestConfig, issues: &mut Vec<String>) {
    if request.url_template.trim().is_empty() {
        issues.push("snapshot.request.url_template must not be empty".to_string());
    }

    match request.method {
        super::HttpMethod::Get => {}
    }
}

fn validate_snapshot_response(response: &SnapshotResponse, issues: &mut Vec<String>) {
    match response {
        SnapshotResponse::Json(json) => {
            if json.data_path.is_empty() {
                issues.push("snapshot.response.path must contain at least one segment".to_string());
            }

            if !json.data_path.iter().any(matches_symbol_segment) {
                issues.push(
                    "snapshot.response.path should reference `{symbol}` for code substitution"
                        .to_string(),
                );
            }
        }
        SnapshotResponse::Delimited(delimited) => {
            if delimited.delimiter == '\0' {
                issues.push(
                    "snapshot.response.delimited delimiter must be a visible character".to_string(),
                );
            }
        }
    }
}

fn matches_symbol_segment(segment: &JsonPathSegment) -> bool {
    matches!(segment, JsonPathSegment::StockCode)
}

fn validate_info_indices(info_idxs: &HashMap<String, InfoIndex>, issues: &mut Vec<String>) {
    if info_idxs.is_empty() {
        issues.push("snapshot.info_indices must define at least one mapping".to_string());
        return;
    }

    let mut seen = HashMap::<usize, String>::new();
    let mut duplicates = Vec::new();

    for (label, idx) in info_idxs {
        if let Some(existing) = seen.insert(idx.index, label.clone()) {
            duplicates.push(format!(
                "index {} assigned to both `{existing}` and `{label}`",
                idx.index
            ));
        }
    }

    if !duplicates.is_empty() {
        issues.push(format!(
            "snapshot.info_indices contains duplicate positions: {}",
            duplicates.join(", ")
        ));
    }
}

fn validate_tencent_history_config(endpoint: &str, issues: &mut Vec<String>) {
    if endpoint.trim().is_empty() {
        issues.push("provider.history.endpoint must not be empty".to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::load_region_descriptor;
    use std::path::Path;

    #[test]
    fn validates_cn_descriptor() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let descriptor = load_region_descriptor(root, "cn").expect("load cn descriptor");

        validate_region_descriptor(&descriptor).expect("cn descriptor should be valid");
    }

    #[test]
    fn rejects_duplicate_info_indices() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut descriptor = load_region_descriptor(root, "cn").expect("load cn descriptor");

        if let ProviderConfig::Tencent(tencent) = &mut descriptor.provider {
            tencent
                .snapshot
                .info_idxs
                .insert("duplicated".to_string(), InfoIndex { index: 1 });
        }

        let err = validate_region_descriptor(&descriptor).expect_err("validation should fail");
        let message = err.to_string();
        assert!(
            message.contains("duplicate positions"),
            "unexpected error message: {message}"
        );
    }

    #[test]
    fn rejects_threshold_with_inverted_bounds() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut descriptor = load_region_descriptor(root, "cn").expect("load cn descriptor");
        descriptor.thresholds.insert(
            "bad_metric".to_string(),
            Threshold {
                lower: 10.0,
                upper: 5.0,
                valid: true,
            },
        );

        let err = validate_region_descriptor(&descriptor).expect_err("validation should fail");
        assert!(
            err.to_string().contains("lower bound"),
            "unexpected error message: {}",
            err
        );
    }

    #[test]
    fn rejects_missing_symbol_segment() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut descriptor = load_region_descriptor(root, "cn").expect("load cn descriptor");

        if let ProviderConfig::Tencent(tencent) = &mut descriptor.provider {
            tencent.snapshot.response =
                SnapshotResponse::Json(JsonResponseConfig { data_path: vec![] });
        }

        let err = validate_region_descriptor(&descriptor).expect_err("validation should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("at least one segment"),
            "unexpected error message: {msg}"
        );
    }
}
