use std::collections::HashMap;

use super::{
    CodeTransform, FirewallWarning, HttpMethod, InfoIndex, JsonPathSegment, JsonResponseConfig,
    ProviderConfig, RegionConfig, RequestConfig, SnapshotConfig, SnapshotResponse,
    TencentHistoryConfig, TencentProviderConfig, Threshold,
};

pub fn region() -> RegionConfig {
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

    let thresholds = HashMap::from([
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

    let snapshot = SnapshotConfig {
        request: RequestConfig {
            method: HttpMethod::Get,
            url_template: "http://ifzq.gtimg.cn/appstock/app/kline/mkline?param={code},m1,,10"
                .to_string(),
            headers,
            code_transform: CodeTransform::default(),
        },
        response: SnapshotResponse::Json(JsonResponseConfig {
            data_path: vec![
                JsonPathSegment::Key("data".to_string()),
                JsonPathSegment::StockCode,
                JsonPathSegment::Key("qt".to_string()),
                JsonPathSegment::StockCode,
            ],
        }),
        info_idxs,
        firewall_warning: Some(FirewallWarning {
            text: "window.location.href=\"https://waf.tencent.com/501page.html?u=".to_string(),
        }),
    };

    let provider = ProviderConfig::Tencent(TencentProviderConfig {
        snapshot,
        history: TencentHistoryConfig {
            endpoint: "https://ifzq.gtimg.cn/appstock/app/kline/kline".to_string(),
            referer: "https://gu.qq.com/".to_string(),
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36".to_string(),
            accept_language: "en-US,en;q=0.9".to_string(),
            record_days: 420,
        },
    });

    RegionConfig {
        code: "CN".to_string(),
        name: "China A-Shares".to_string(),
        stock_code_file: "assets/.markets/cn.csv".to_string(),
        thresholds,
        provider,
    }
}
