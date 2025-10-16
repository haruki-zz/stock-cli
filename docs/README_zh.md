# Stock CLI

Stock CLI 是一个专注中国 A 股的终端行情助手。它调用腾讯行情接口抓取实时数据，按时间戳写入 CSV 快照，并依托 Ratatui 构建的多界面 TUI 提供筛选、排序与蜡烛图联动浏览。当前聚焦中国市场，同时保留未来引入其他区域的扩展点。

![应用主菜单](../img/main_menu.png)

## 核心能力
- **实时行情**：读取 `stock_code.csv` 中的股票代码，批量请求现价、涨跌幅、幅度、换手率等指标。
- **条件筛选**：为每个指标设置阈值范围，支持保存/加载本地预设，快速复用常用策略。
- **历史走势**：为当前选中股票抓取最长 420 天的日线 K 线，在结果页内切换多种时间窗口。
- **快照管理**：每次抓取都会在 `assets/snapshots/cn/` 下生成带时间戳的 CSV，方便回放以往数据。

## 快速上手
- **准备工具链**：安装稳定版 Rust（含 `cargo`）。
- **获取代码**：`git clone <repo-url> && cd stock-cli`
- **编译程序**：`cargo build --release`
- **准备股票清单**：在可执行文件同级目录放置 `stock_code.csv`（每行一个代码，仅限中国市场）。若要增加其他区域，可在 `assets/.markets/` 中新增对应 CSV 并在配置中注册。
- **启动应用**：`./target/release/stock-cli`（开发阶段可使用 `cargo run`）。

## 界面使用指南
- **主菜单条目**
  - `Show Filtered`：查看符合阈值的股票列表。`s` 切换排序列，`d` 切换升/降序，Enter 展开蜡烛图。
  - `Filters`：编辑上下限、保存或加载阈值预设。Tab 或方向键切换输入框，Enter 保存。
  - `Refresh Data`：抓取最新行情并写入 CSV。
  - `Load CSV`：从 `assets/snapshots/` 选择历史快照回放。
  - `Quit`：退出程序。（当注册多个区域时会自动出现 `Switch Market` 选项。）
- **导航速查**
  - 方向键或 `j/k`：移动选项
  - `Enter`：确认
  - `Esc`：返回
  - `Ctrl+C`：随时退出
- **推荐流程**
  1. 在主菜单选择 `Refresh Data` 拉取当日行情；
  2. 进入 `Filters` 调整筛选条件；
  3. 在 `Show Filtered` 中浏览结果并查看联动图；
  4. 依据需要保存预设或加载历史快照。

## 数据布局与扩展
- **股票清单**：`stock_code.csv`（必需）。新增区域时，请在 `assets/.markets/<region>.csv` 中维护代码表。
- **快照文件**：`assets/snapshots/<region>/`。
- **阈值预设**：`assets/filters/<region>/`。
- **扩展新市场**：复制 `docs/examples/sample_region.json` 为 `assets/configs/<region>.json`，按格式完善接口/阈值，再准备 `assets/.markets/<region>.csv`，重新加载市场即可生效。
- **新增市场示例**：参考 `docs/examples/sample_region.json`，复制为 `assets/configs/<地区>.json`，并同时提供 `assets/.markets/<地区>.csv` 即可。示例展示了自定义存储目录、快照/历史接口模板以及 JSON 响应字段映射。
