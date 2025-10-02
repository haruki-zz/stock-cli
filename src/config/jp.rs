use std::collections::HashMap;

use super::{
    ProviderConfig, RegionConfig, StooqHistoryConfig, StooqProviderConfig, StooqSnapshotConfig,
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

    let provider = ProviderConfig::Stooq(StooqProviderConfig {
        symbol_suffix: ".jp".to_string(),
        snapshot: StooqSnapshotConfig {
            quote_endpoint: "https://stooq.com/q/l/".to_string(),
        },
        history: StooqHistoryConfig {
            endpoint: "https://stooq.com/q/d/l/".to_string(),
        },
        listings_url: "https://www.jpx.co.jp/english/markets/statistics-equities/misc/tvdivq0000001vg2-att/jyoujyou(updated)_e.xlsx".to_string(),
    });

    RegionConfig {
        code: "JP".to_string(),
        name: "Japan Stocks".to_string(),
        stock_code_file: "stock_codes/japan.csv".to_string(),
        thresholds,
        provider,
    }
}
