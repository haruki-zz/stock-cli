# Repository Guidelines

## Overview
- `src/main.rs` 调用 `app::bootstrap`，初始化 `MarketRegistry`、加载配置驱动的 `RegionDescriptor`，并启动 Ratatui UI
- 核心模块：`src/app`（控制流与市场切换）、`src/config`（CSV/JSON 描述符加载）、`src/fetch`（快照与历史抓取）、`src/records`（CSV/预设持久化）、`src/ui`（组件与流程）、`src/utils`（通用工具）、`src/error.rs`（错误枚举）
- 每个市场由 `assets/.markets/<code>.csv` 与 `assets/configs/<code>.json` 定义；运行时无需改动源码即可新增。
- `AppController` 和 UI 仅与 `RegionDescriptor` 交互，确保数据来源可被热加载与验证。

## Market Assets
- `.markets/<code>.csv`：列出可查询的股票代码、名称以及额外元数据列（顺序在配置中声明）。
- `configs/<code>.json`：声明快照/历史接口、请求模板、响应字段映射、默认阈值与展示偏好。
- `filters/<code>/` 与 `snapshots/<code>/`：仍按区域分目录写入，保持向后兼容性。
- 新增市场流程：复制示例 CSV/JSON，调整字段后触发“重新加载市场”即可。

## Behaviour Highlights
- Snapshots：并发抓取、自动判别防火墙错误，依据 JSON 描述的 `response` 映射解析字段，并写入 `snapshots/<code>/timestamp.csv`。
- History：使用同一描述符生成请求，支持 OHLCV、JSON path 或自定义解码方式，驱动多时间范围图表。
- Threshold editor：读取配置提供的指标与默认值；保存仍写入 `filters/<code>/preset.json`。
- Market selection：UI 通过 `MarketRegistry` 实时拉取可用市场，支持热加载或配置失败提示。

## Development Commands
- `cargo fmt` — formatting（提交前运行）。
- `cargo clippy -- -D warnings` — lint gate。
- `cargo test` — 单元 + 集成测试。
- `cargo run` — 启动 TUI（含调试日志）。
- `cargo build --release` — 生成产物 `target/release/stock-cli`。
- `./build_macos_intel_release.sh` — 可选交叉编译脚本。

## Coding & Style
- 遵循 `docs/coding_principles.md`：防止跨层依赖、清晰错误路径、首选 async/await。
- UI 侧尽量复用 `src/ui/styles.rs` 提供的 `header_text`、`secondary_line`、`selection_style`、`ACCENT` 等链式样式工具。
- 在 `src/config` 内集中定义 serde schema 和验证逻辑；不要在 `fetch` 层重新解析 JSON 字段名称。
- 新增共享工具放在就近的 `mod.rs` 下，保持模块单一职责。
- 序列化阈值前调用 `ensure_metric_thresholds`，保证 UI/预设一致。

## Testing & Assets
- 为配置解析、请求生成、解码器转换编写针对性测试（优先表驱动）。
- 集成测试可以模拟加载内置示例 JSON，确认快照/历史解析与数据表头一致。
- 仅在 `assets/snapshots/<code>/`、`assets/filters/<code>/` 写入运行时文件，避免污染仓库。
- 更新任何流程或模块后，同步维护 `docs/architecture.md`、`docs/styles.md` 与示例配置。
