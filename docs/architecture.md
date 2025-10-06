# stock-cli 架构说明

## 目录结构
```text
assets/
  .markets/
    cn.csv
    jp.csv
  filters/
    cn/
    jp/
  snapshots/
    cn/
    jp/

src/
  main.rs
  app/
    mod.rs
    bootstrap.rs
    controller.rs
    state.rs
  config/
    mod.rs
    CN.rs
    JP.rs
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
      prompt.rs
      terminal.rs
      helpers.rs
    flows/
      mod.rs
      csv_picker.rs
      fetch_progress.rs
      main_menu.rs
      results.rs
      thresholds.rs
      preset_picker.rs
      preset_save.rs
  utils/
    mod.rs
    file.rs
    text.rs
    time.rs
  error.rs
```

## 模块说明

### assets/.markets
- `cn.csv`: 中国股票市场的股票代码。
- `jp.csv`: 日本股票市场的股票代码

### assets/filters/cn
- 用于保存针对中国市场的filter文件

### assets/filters/jp
- 用于保存针对日本市场的filter文件

### assets/snapshots/cn
- 用于保存中国市场的snapshots（CSV文件）

### assets/snapshots/jp
- 用于保存日本市场的snapshots（CSV文件）

### src/main.rs
- 程序入口，解析命令行参数、配置日志并调用 `app::bootstrap`。

### src/app/
- `mod.rs`：提供启动和关闭 TUI 的对外接口。
- `bootstrap.rs`：组装配置、抓取服务、记录层与 UI，最后交给控制器运行。
- `controller.rs`：协调实时快照、历史下载与 UI 触发的动作。
- `state.rs`：集中管理跨线程共享的应用状态。

### src/config/
- `mod.rs`：对外接口。
- `CN.rs`: 内置中国市场的 HTTP 请求元数据与指标阈值默认值。
- `JP.rs`: 内置日本市场的 HTTP 请求元数据与指标阈值默认值。

### src/fetch/
- `mod.rs`：抓取逻辑共用的 trait、错误别名与辅助函数。
- `snapshots.rs`：异步快照抓取器，驱动进度视图并生成最新的 CSV 数据集。
- `history.rs`：历史数据抓取任务，下载所选股票最长 420 天的 K 线并解析。

### src/records/
- `mod.rs`：对外暴露数据记录接口。
- `stock_database.rs`：负责 CSV 持久化、最新文件查找与内存视图构建。
- `presets.rs`：读写 UI 中选择的阈值预设。

### src/ui/
- `mod.rs`：初始化 Ratatui 终端并实现主事件循环。
- `styles.rs`：共享色彩、字符和留白规则，遵循 `doc/styles.md`。
- `navigation.rs`：定义当前路由枚举，帮助渲染循环在各流程间切换。
- `components/`：可复用控件（图表、表格、提示框、终端初始化、格式化辅助）。
- `flows/`：具体界面流程（CSV 选择器、抓取进度、主菜单、结果、阈值编辑、预设选择与保存）。

### src/utils/
- `mod.rs`：导出常用工具函数。
- `file.rs`：抓取与记录层共用的文件系统工具。
- `text.rs`：字符串格式化工具，包括状态标签与数值对齐。
- `time.rs`：时间换算与交易日辅助函数，供抓取和 UI 共用。

### src/error.rs
- 全局错误枚举与 `Result` 别名，统一抓取、记录层、UI 的错误处理。
