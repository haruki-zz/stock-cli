# stock-cli 架构说明

## 目录结构
```text
assets/
  .markets/
    cn.csv
  filters/
    cn/
  snapshots/
    cn/

src/
  main.rs
  app/
    mod.rs
    bootstrap.rs
    controller.rs
    state.rs
  config/
    mod.rs
    cn.rs
  fetch/
    mod.rs
    snapshots.rs
    history.rs
  records/
    mod.rs
    stock_database.rs
    presets.rs
  ui/
    mod.rs
    styles.rs
    navigation.rs
    components/
      mod.rs
      chart.rs
      table.rs
      terminal.rs
      utils.rs
    flows/
      mod.rs
      csv_picker.rs
      fetch_progress.rs
      main_menu.rs
      results.rs
      market_picker.rs
      preset_picker.rs
      thresholds.rs
  utils/
    mod.rs
    file.rs
    text.rs
    time.rs
  error.rs
```

## 模块说明

### assets/.markets
- `cn.csv`: 中国股票市场的股票代码。若新增市场，可在该目录下按区域代码扩展 CSV 并同步代码模块。

### assets/filters/
- `cn/`: 用于保存针对中国市场的 filter 文件。

### assets/snapshots/
- `cn/`: 用于保存中国市场的 snapshots（CSV 文件）。

### src/main.rs
- 程序入口，解析命令行参数、配置日志并调用 `app::bootstrap`。

### src/app/
- `mod.rs`：提供启动和关闭 TUI 的对外接口。
- `bootstrap.rs`：组装配置、抓取服务、记录层与 UI，最后交给控制器运行。
- `controller.rs`：协调实时快照、历史下载与 UI 触发的动作。
- `state.rs`：集中管理跨线程共享的应用状态。

### src/config/
- `mod.rs`：对外接口。
- `cn.rs`: 内置中国市场的 HTTP 请求元数据与指标阈值默认值。
- 在新增市场时，可在该目录下增加模块并在 `mod.rs` 中注册，以复用同一抓取和持久化管线。

### src/fetch/
- `mod.rs`：抓取逻辑共用的类型与并发限制工具。
- `snapshots.rs`：异步快照抓取器，生成最新的 CSV 数据集，并在被腾讯防火墙拦截时给出错误提示。
- `history.rs`：历史数据抓取任务，调用腾讯接口解析至多 420 天的日线并向图表视图广播。

### src/records/
- `mod.rs`：对外暴露数据记录接口。
- `stock_database.rs`：负责 CSV 持久化、最新文件查找与内存视图构建。
- `presets.rs`：读写阈值预设文件（JSON）。

### src/ui/
- `mod.rs`：初始化 Ratatui 终端并实现主事件循环。
- `styles.rs`：共享色彩、字符和留白规则，遵循 `doc/styles.md`。
- `navigation.rs`：定义当前路由枚举，帮助渲染循环在各流程间切换。
- `components/`：可复用控件（图表、表格、终端初始化、布局与格式化辅助）。
- `flows/`：具体界面流程，包括主菜单、阈值编辑/保存、结果表格与图表、预设选择器、市场/CSV 选择器以及抓取进度视图。

### 运行流程概览
- 启动：`main.rs` 通过 `app::bootstrap` 构建 `AppController`，加载区域配置并准备持久化目录。
- 主菜单：`ui::flows::main_menu` 展示入口操作。用户可刷新行情、浏览过滤结果或加载历史 CSV。
- 过滤阈值：`ui::flows::thresholds` 提供列表与弹窗编辑器，支持在同一界面输入上下限、启用/禁用指标，并通过 `S/Ctrl+S` 打开预设命名对话框后即时保存为 JSON。
- 数据抓取：`fetch::snapshots` 并发请求行情并写入最新 CSV；`fetch::history` 调用腾讯历史接口，为图表准备蜡烛图数据。
- 结果展示：`ui::flows::results` 渲染可排序表格与内嵌多时间范围蜡烛图，按需拉取历史数据并在表格与图表间联动滚动位置。

### src/utils/
- `mod.rs`：导出常用工具函数。
- `file.rs`：抓取与记录层共用的文件系统工具。
- `text.rs`：字符串格式化工具，包括状态标签与数值对齐。
- `time.rs`：时间换算与交易日辅助函数，供抓取和 UI 共用。

### src/error.rs
- 全局错误枚举与 `Result` 别名，统一抓取、记录层、UI 的错误处理。
