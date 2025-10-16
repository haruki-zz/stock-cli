# stock-cli 架构说明

## 设计目标
- 以「CSV + JSON 描述符」为唯一事实来源，做到新增市场无需编译源码即可接入。
- 将抓取、解析、持久化与展示层解耦，使它们只依赖统一的 `RegionDescriptor` 数据模型。
- 通过配置验证与缓存，确保用户自定义市场在运行时能被安全加载、快速切换。
- 复用现有的 UI 流程与文件目录结构，让多市场支持不会扰动已有工作流。

## 目录结构（重构后）
```text
assets/
  .markets/
    <region>.csv         # 股票代码清单，用户可自行添加
  configs/
    <region>.json        # 市场配置描述符，定义抓取端点与解析方式
  filters/
    <region>/            # 用户保存的阈值预设
  snapshots/
    <region>/            # 抓取得到的快照 CSV

src/
  main.rs
  app/
    mod.rs
    bootstrap.rs
    controller.rs
    market_registry.rs   # 管理可用市场、路由选择逻辑
    state.rs
  config/
    mod.rs
    loader.rs            # 读取 CSV/JSON 并转为 RegionDescriptor
    schema.rs            # serde 定义与默认值
    registry.rs          # 缓存、热更新、事件广播
    validator.rs         # 静态检查与错误汇报
  fetch/
    mod.rs
    request.rs           # 构建 HTTP 请求（method、header、query 模板）
    decode.rs            # 通用解析器，使用 JSON 映射提取字段
    snapshots.rs         # 按配置生成 snapshot 数据集
    history.rs           # 按配置生成历史 K 线
  records/
    mod.rs
    stock_database.rs
    presets.rs
  ui/
    mod.rs
    styles.rs
    navigation.rs
    components/
      ...
    flows/
      ...
  utils/
    mod.rs
    file.rs
    text.rs
    time.rs
  error.rs
```

## 市场描述符（RegionDescriptor）

每个市场由一对文件定义：
- `assets/.markets/<region>.csv`：股票代码、名称等基本信息。
- `assets/configs/<region>.json`：抓取端点字段映射、阈值默认值、展示偏好等元数据。

运行时，`config::loader` 会读取两份文件，生成 `RegionDescriptor`（内含快照请求、历史请求、列定义、默认阈值等），并供各层消费。

### JSON 结构草案（以中国市场为例）
```json
{
  "code": "CN",
  "name": "China A-Shares",
  "stock_list": {
    "file": "assets/.markets/cn.csv"
  },
  "thresholds": {
    "amp":   { "lower": 3.0,  "upper": 6.0,   "enabled": false },
    "turnOver": { "lower": 5.0,  "upper": 10.0,  "enabled": true  },
    "tm":    { "lower": 50.0, "upper": 120.0, "enabled": true  },
    "increase": { "lower": 3.0,  "upper": 5.0,   "enabled": true  }
  },
  "provider": {
    "type": "tencent",
    "snapshot": {
      "request": {
        "method": "GET",
        "url_template": "http://ifzq.gtimg.cn/appstock/app/kline/mkline?param={symbol},m1,,10",
        "headers": {
          "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)...",
          "Referer": "http://ifzq.gtimg.cn/appstock/app/kline",
          "Accept-Language": "en-US,en;q=0.9"
        },
        "code_transform": "default"
      },
      "response": {
        "type": "json_path",
        "path": ["data", "{symbol}", "qt", "{symbol}"]
      },
      "firewall_warning": "window.location.href=\"https://waf.tencent.com/501page.html?u=",
      "info_indices": {
        "stockName": 1,
        "stockCode": 2,
        "curr": 3,
        "prevClosed": 4,
        "open": 5,
        "increase": 32,
        "highest": 33,
        "lowest": 34,
        "turnOver": 38,
        "amp": 43,
        "tm": 44
      }
    },
    "history": {
      "endpoint": "https://ifzq.gtimg.cn/appstock/app/kline/kline",
      "headers": {
        "Referer": "https://gu.qq.com/",
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)...",
        "Accept-Language": "en-US,en;q=0.9"
      },
      "record_days": 420
    }
  }
}
```

关键字段说明：
- `provider.type`：声明抓取提供方，实现端可据此选择特定解析策略（如腾讯专用的历史接口）。
- `snapshot.request`：构造 HTTP 请求所需的 method、URL 模板、头信息及代码转换规则（`default` 表示按 CSV 代码原样拼接）。
- `snapshot.response.path`：JsonPath 数组，支持占位符 `{symbol}` 表示当前股票代码。
- `info_indices`：对腾讯快照数组字段的下标映射，驱动 UI 展示与指标计算。
- `history.record_days`：一次抓取的最大日线数量，可由不同市场配置。
- `thresholds`：默认筛选上下限与是否启用，启动时会结合用户预设调用 `ensure_metric_thresholds`。

## 模块职责

### src/app
- `bootstrap`：加载配置目录、初始化 `MarketRegistry` 和共享状态。
- `controller`：驱动抓取与 UI 流程，监听市场切换事件。
- `market_registry`：管理可用市场列表、热加载更新、提供 UI/控制层检索接口。
- `state`：保存当前市场、快照缓存、阈值设置等。

### src/config
- `loader`：解析 CSV、JSON，执行占位符替换、构建 `RegionDescriptor`。
- `schema`：集中 serde 结构体定义，提供默认值与版本迁移策略。
- `registry`：缓存已加载配置，监听文件系统变化（如用户添加新市场），暴露观察者接口。
- `validator`：校验字段缺失、类型不匹配、URL 非法等问题，并返回 `AppError::InvalidConfig`。
- `mod.rs`：对外提供 `load_region`, `available_regions`, `reload_all` 等入口。

### src/fetch
- `request`：根据 `RegionDescriptor` 构建请求（HTTP method、header、query、body）。
- `snapshots` / `history`：执行抓取任务，应用并发与重试策略。
- `decode`：通用解析器，负责按配置提取字段、应用转换函数（如 decimal、percent、split OHLC）。
- 模块不再依赖硬编码的腾讯字段，而是通过 `RegionDescriptor` 描述的抽象访问数据。

### src/records
- 维持现有职责：快照 CSV 写入、最近文件加载、预设 JSON 管理。
- 加入基于 `RegionDescriptor` 的路径路由逻辑（按 `region` 写入相应目录）。

### src/ui
- UI 继续使用 Ratatui，额外消费 `MarketRegistry` 提供的市场列表与阈值默认值。
- 阈值编辑器保存时引用 `RegionDescriptor` 的指标集并调用 `ensure_metric_thresholds` 归一化。

### src/utils / src/error.rs
- 扩展文件与 JSON 工具，支持配置热加载、路径拼接。
- 为配置相关错误提供细分枚举（如 `MissingField`, `InvalidPlaceholder`, `DecodeFailure`）。

## 运行流程
1. **启动阶段**：`app::bootstrap` 读取 `assets/configs` 与 `assets/.markets`，构建 `MarketRegistry` 并注入初始 `RegionDescriptor`。
2. **市场选择**：用户通过 UI 选择市场时，`MarketRegistry` 返回匹配的描述符；若文件更新则自动刷新缓存。
3. **快照抓取**：`fetch::snapshots` 使用描述符生成请求，解析响应，写入 `assets/snapshots/<region>/timestamp.csv`，并更新内存状态。
4. **历史抓取**：用户打开图表时触发 `fetch::history`，按配置拉取 K 线数据，复用同一解析器。
5. **阈值与预设**：UI 从 `RegionDescriptor` 中获取指标集合与默认值，用户保存时写入 `assets/filters/<region>/`。
6. **错误处理**：配置解析/验证失败会直接反馈至 UI，并给出缺失字段、非法占位符等具体提示。

## 扩展新市场的步骤
1. 在 `assets/.markets/` 复制示例 CSV，填入目标市场的代码、名称等字段。
2. 在 `assets/configs/` 新建 `<region>.json`，参照示例指定快照与历史接口、字段映射、默认阈值。
3. 重新启动程序或在 UI 内触发“重新加载市场”动作；若配置合法，会自动出现在市场选择列表中。
4. 无需修改任何 Rust 代码，即可完成新市场接入。
