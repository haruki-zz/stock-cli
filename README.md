# Stock CLI

终端内的中国股票市场快照与历史行情工具。程序抓取腾讯行情接口、落盘 CSV，并通过 Ratatui 构建的多界面 TUI 提供筛选、排序与走势图。未来仍可扩展其他区域，当前默认聚焦中国 A 股。

![应用主菜单](./img/main_menu.png)

## 核心能力
- 获取最新行情：读取 `stock_code.csv` 中的股票代码，批量请求实时数据并展示关键信息（现价、涨跌幅、换手率等）。
- 结果筛选：设置阈值过滤器，快速定位满足成交额、涨幅等条件的标的，可保存/加载本地预设。
- 历史走势：为当前选中股票加载最长 420 天 K 线，并在结果页内联动展示多周期蜡烛图。
- CSV 管理：每次抓取自动输出带时间戳的快照至 `assets/snapshots/cn/`，支持在程序内快速回放旧数据。

## 快速上手
- 安装依赖：准备稳定版 Rust 工具链（含 `cargo`）。
- 获取代码：`git clone <repo-url> && cd stock-cli`
- 构建应用：`cargo build --release`
- 准备数据：在可执行文件同级目录放置 `stock_code.csv`（一行一个代码，仅限中国市场）；需要拓展其他市场时，可在 `assets/.markets/` 下新增区域 CSV 并在配置中注册。
- 启动程序：`./target/release/stock-cli`（开发过程中可使用 `cargo run`）。

## 使用指南
- 主菜单
  - `Show Filtered`：查看符合当前阈值的股票列表，支持 `s`（切换排序列）、`d`（正逆序）、Enter（展开 K 线图）、`←/→`（切换时间范围）。
  - `Filters`：调整、保存或加载阈值。编辑器支持 Tab/方向键在上下限间切换，输入数值后 Enter 保存。
  - `Refresh Data`：立即抓取最新行情并落盘。
  - `Load CSV`：从本地快照目录选择历史数据集回放。
  - `Quit`：退出程序。（当存在多个区域时，会自动出现 `Switch Market` 选项。）
- 导航习惯
  - 方向键或 `j/k`：移动选项
  - `Enter`：确认，`Esc`：返回上一层，`Ctrl+C`：强制退出
- 建议流程
  1. 在主菜单选择 `Refresh Data` 获取最新行情；
  2. 进入 `Filters` 调整筛选条件；
  3. 在 `Show Filtered` 中浏览结果、查看联动图表；
  4. 根据需要保存阈值或加载旧 CSV。

## 数据与扩展
- 行情代码：`stock_code.csv`（必需），自定义区域时请在 `assets/.markets/` 下新增对应文件。
- 快照输出：`assets/snapshots/<region>/`.
- 阈值预设：`assets/filters/<region>/`.
- 新市场扩展：在 `src/config/` 下新增区域模块并在 `Config::builtin` 中注册即可复用现有流程。
