use std::collections::HashMap;

use super::{
    CodeTransform, DelimitedResponseConfig, HttpMethod, InfoIndex, ProviderConfig, RegionConfig,
    RequestConfig, SnapshotConfig, SnapshotResponse, StooqHistoryConfig, StooqProviderConfig,
    Threshold,
};

pub fn region() -> RegionConfig {
    let thresholds = HashMap::from([
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

    let info_idxs = HashMap::from([
        ("curr".to_string(), InfoIndex { index: 6 }),
        ("prevClosed".to_string(), InfoIndex { index: 7 }),
        ("open".to_string(), InfoIndex { index: 3 }),
        ("highest".to_string(), InfoIndex { index: 4 }),
        ("lowest".to_string(), InfoIndex { index: 5 }),
        ("volume".to_string(), InfoIndex { index: 8 }),
    ]);

    let snapshot = SnapshotConfig {
        request: RequestConfig {
            method: HttpMethod::Get,
            url_template: "https://stooq.com/q/l/?s={code}&f=sd2t2ohlcpv&h=1&e=csv".to_string(),
            headers: HashMap::new(),
            code_transform: CodeTransform {
                lowercase: true,
                uppercase: false,
                prefix: None,
                suffix: Some(".jp".to_string()),
            },
        },
        response: SnapshotResponse::Delimited(DelimitedResponseConfig {
            delimiter: ',',
            skip_lines: 1,
        }),
        info_idxs,
        firewall_warning: None,
    };

    let provider = ProviderConfig::Stooq(StooqProviderConfig {
        symbol_suffix: ".jp".to_string(),
        snapshot,
        history: StooqHistoryConfig {
            endpoint: "https://stooq.com/q/d/l/".to_string(),
        },
    });

    RegionConfig {
        code: "JP".to_string(),
        name: "Japan Stocks".to_string(),
        stock_code_file: "assets/.markets/jp.csv".to_string(),
        thresholds,
        provider,
    }
}
