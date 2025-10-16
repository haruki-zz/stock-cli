# Multi-Market Support Roadmap

- [x] Introduce `assets/configs/<region>.json` alongside `.markets/<region>.csv`, including migration tooling for现有 `cn` 市场。（see `assets/configs/cn.json` 与 `scripts/migrate_cn_config.py`）
- [x] Implement `config::loader` to parse CSV/JSON、执行占位符替换，并返回统一的 `RegionDescriptor`。（see `src/config/loader.rs`，`config::load_region_descriptor`）
- [x] Add `config::validator` 覆盖字段存在性、URL/HTTP method 合法性、列映射重复等校验，清晰汇报错误。（see `src/config/validator.rs` and loader integration）
- [x] Build `config::registry` 和 `app::market_registry`，支持缓存、热加载、以及面向 UI 的观察者 API。（see `src/config/registry.rs`、`src/app/market_registry.rs`、`src/app/controller.rs`）
- [x] Refactor `fetch::snapshots` / `fetch::history` 通过配置构造请求与解析响应，移除对腾讯字段的硬编码。（see `src/fetch/request.rs`, `src/fetch/snapshots.rs`, `src/fetch/history.rs`, updated config structs）
- [x] Extend `fetch::decode`（或新增模块）以支持 JSON path、分隔符转换、数值/日期格式化等声明式解析。（see `src/fetch/decode.rs`, updated `src/fetch/snapshots.rs`, `src/fetch/history.rs`）
- [x] Update `records` 层根据 `RegionDescriptor` 决定输出目录，保持现有快照与预设 API 行为不变。（see `src/config/mod.rs`, `src/config/loader.rs`, `src/records/mod.rs`）
- [x] Wire UI 市场选择器到 `MarketRegistry`，并确保阈值编辑器读取配置中的指标/默认值，同时监听 registry 更新以便热刷新。（see `src/app/controller.rs`, `src/ui/components/chart.rs`, `src/ui/flows/results.rs`）
- [x] Add 回归 测试：最小端到端跑通配置加载、快照抓取模拟、阈值保存路由（可通过feature或集成测试实现）。（see `tests/regression.rs`, `src/lib.rs`)
- [x] 编写面向用户的示例配置（`docs/examples/<region>.json`），说明新增市场的必填字段与约束。（see `docs/examples/sample_region.json`, README updates）
