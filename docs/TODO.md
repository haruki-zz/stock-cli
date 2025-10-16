# Multi-Market Support Roadmap

- [ ] Introduce `assets/configs/<region>.json` alongside `.markets/<region>.csv`, including migration tooling for现有 `cn` 市场。
- [ ] Implement `config::loader` to parse CSV/JSON、执行占位符替换，并返回统一的 `RegionDescriptor`。
- [ ] Add `config::validator` 覆盖字段存在性、URL/HTTP method 合法性、列映射重复等校验，清晰汇报错误。
- [ ] Build `config::registry` 和 `app::market_registry`，支持缓存、热加载、以及面向 UI 的观察者 API。
- [ ] Refactor `fetch::snapshots` / `fetch::history` 通过配置构造请求与解析响应，移除对腾讯字段的硬编码。
- [ ] Extend `fetch::decode`（或新增模块）以支持 JSON path、分隔符转换、数值/日期格式化等声明式解析。
- [ ] Update `records` 层根据 `RegionDescriptor` 决定输出目录，保持现有快照与预设 API 行为不变。
- [ ] Wire UI 市场选择器到 `MarketRegistry`，并确保阈值编辑器读取配置中的指标/默认值。
- [ ] Add 回归 测试：最小端到端跑通配置加载、快照抓取模拟、阈值保存路由（可通过feature或集成测试实现）。
- [ ] 编写面向用户的示例配置（`docs/examples/<region>.json`），说明新增市场的必填字段与约束。
