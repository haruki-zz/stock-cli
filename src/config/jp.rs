use std::collections::HashMap;

use super::{
    CodeTransform, HistoryProviderKind, HttpMethod, InfoIndex, JsonPathSegment, JsonResponseConfig,
    ProviderConfig, RegionConfig, RequestConfig, SnapshotConfig, SnapshotResponse,
    TencentHistoryConfig, TencentProviderConfig, Threshold,
};

pub const JP_REGION_CODE: &str = "JP";
pub const JP_REGION_NAME: &str = "Japan Prime Market";

const SNAPSHOT_URL: &str = "https://api.jquants.com/v1/pricing/snapshot?code={code}";
const HISTORY_URL: &str = "https://api.jquants.com/v1/prices/daily_quotes";

pub fn region() -> RegionConfig {
    let info_idxs = HashMap::from([
        ("stockCode".to_string(), InfoIndex { index: 0 }),
        ("stockName".to_string(), InfoIndex { index: 1 }),
        ("curr".to_string(), InfoIndex { index: 2 }),
        ("prevClosed".to_string(), InfoIndex { index: 3 }),
        ("open".to_string(), InfoIndex { index: 4 }),
        ("highest".to_string(), InfoIndex { index: 5 }),
        ("lowest".to_string(), InfoIndex { index: 6 }),
        ("increase".to_string(), InfoIndex { index: 7 }),
        ("amp".to_string(), InfoIndex { index: 8 }),
        ("volume".to_string(), InfoIndex { index: 9 }),
        ("turnOver".to_string(), InfoIndex { index: 10 }),
        ("tm".to_string(), InfoIndex { index: 11 }),
    ]);

    let thresholds = HashMap::from([
        (
            "increase".to_string(),
            Threshold {
                lower: 1.5,
                upper: 4.5,
                valid: true,
            },
        ),
        (
            "turnOver".to_string(),
            Threshold {
                lower: 1.0,
                upper: 4.0,
                valid: true,
            },
        ),
        (
            "tm".to_string(),
            Threshold {
                lower: 20.0,
                upper: 80.0,
                valid: true,
            },
        ),
        (
            "amp".to_string(),
            Threshold {
                lower: 2.0,
                upper: 6.0,
                valid: false,
            },
        ),
    ]);

    let headers = HashMap::from([
        ("Accept".to_string(), "application/json".to_string()),
        (
            "Authorization".to_string(),
            "Bearer ${JQUANTS_TOKEN}".to_string(),
        ),
        (
            "User-Agent".to_string(),
            "stock-cli/0.1 (+https://github.com/haruki/stock-cli)".to_string(),
        ),
    ]);

    let snapshot = SnapshotConfig {
        request: RequestConfig {
            method: HttpMethod::Get,
            url_template: SNAPSHOT_URL.to_string(),
            headers,
            code_transform: CodeTransform::default(),
        },
        response: SnapshotResponse::Json(JsonResponseConfig {
            data_path: vec![
                JsonPathSegment::Key("quotes".to_string()),
                JsonPathSegment::Index(0),
            ],
        }),
        info_idxs,
        firewall_warning: None,
    };

    let provider = ProviderConfig::Tencent(TencentProviderConfig {
        snapshot,
        history: TencentHistoryConfig {
            endpoint: HISTORY_URL.to_string(),
            referer: "https://jpx-jquants.com/".to_string(),
            user_agent: "stock-cli/0.1 (+https://github.com/haruki/stock-cli)".to_string(),
            accept_language: "ja,en;q=0.9".to_string(),
            record_days: 420,
            auth_header: Some("Bearer ${JQUANTS_TOKEN}".to_string()),
            kind: HistoryProviderKind::Jquants,
        },
    });

    RegionConfig {
        code: JP_REGION_CODE.to_string(),
        name: JP_REGION_NAME.to_string(),
        stock_code_file: "assets/.markets/jp.csv".to_string(),
        thresholds,
        provider,
    }
}
