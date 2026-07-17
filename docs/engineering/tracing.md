# 结构化日志与链路

> 代码：`code/engineering-tracing/src/main.rs`　运行：`cargo run -p engineering-tracing`（试 `curl "http://127.0.0.1:7080/order?user_id=42&amount=99"`，  
> 再用 `RUST_LOG=debug` 重跑对比）

[《中间件与优雅退出》](../http/middleware-shutdown.md) 埋的伏笔（「生产建议用 TraceLayer + tracing」）在本课兑现——从此告别 println/logln 式日志。

----

# println 的天花板

> 本书一直用 `logln!`（时间戳 + 文本），教学够用，生产有三个硬伤。

生产硬伤：没有级别过滤、没有结构化字段（没法按 user_id 检索）、没有请求上下文（两条并发请求的日志混在一起分不清谁是谁）。

tracing 生态分三个角色（Go 对照）：

| 角色 | crate | 干什么 | Go 对照 |
| --- | --- | --- | --- |
| 门面 | `tracing` | 代码里打日志的宏：`info!/warn!` + span 概念 | slog/zap 的 API 面 |
| 订阅端 | `tracing-subscriber` | 日志去哪、什么格式、过滤规则 | slog 的 Handler / zap 的 Core |
| HTTP 接入 | `tower-http` 的 TraceLayer | 每个请求自动一个 span | gin.Logger()，但更强 |

门面和订阅端分离的好处和 Go 的 slog 同理：库作者只依赖门面打日志，进不进文件/JSON/OTLP 由应用的订阅端统一决定。

----

# 事件是一条日志

> event（事件）= 结构化地打一条日志。字段是字段、消息是消息。

```rust
info!(user_id = 42, amount = 99, "收到下单请求");
//    ^^^^^^^^^^^^^^^^^^^^^^^^^ 结构化字段        ^^^ 人读的消息
warn!(err = ?e, "调用下游失败");     // ?e：按 Debug 格式记录
info!(addr = %peer, "新连接");       // %peer：按 Display 格式记录
```

- 五个级别：`trace! < debug! < info! < warn! < error!`；
- **字段是字段、消息是消息** ——别再 `format!("user_id={}...", id)` 拼进消息里：  
  结构化字段在 JSON 输出/日志平台里是可检索的列；
- 对照 Go：`slog.Info("收到下单请求", "user_id", 42)` / zap 的 `zap.Int(...)`——同一个思想，  
  Rust 用宏做到了零成本（级别被过滤时连参数求值都跳过）。

----

# span 是请求上下文

> 本课核心概念：**event 是一个时刻，span 是一段时间**。「处理这个请求」从开始到结束是一个 span，期间发生的所有 event 都归属于它。

为什么异步世界必须要 span：Go 里你用 `ctx` 手动把 request-id/user_id 层层传下去、  
每条日志手动带上；线程世界还能靠 goroutine-local 偷懒。而 Tokio 的任务会在线程间搬家（工作窃取），  
「线程本地存储」根本靠不住——tracing 的 span 是 **跟着 Future 走的上下文**：进入 span 范围内打的日志自动携带 span 的全部字段。

```rust
#[instrument(skip(params), fields(user_id = params.user_id))]
async fn create_order(Query(params): Query<OrderParams>) -> Json<OrderResp> {
    info!(amount = params.amount, "收到下单请求");   // ← 自动带 user_id
    let order_id = save_order(params.amount).await;  // ← save_order 里的日志也带！
    ...
}

#[instrument]
async fn save_order(amount: i64) -> u64 { debug!("开始写库"); ... }
```

跑起来看输出（缩略）：

```
INFO create_order{user_id=42}: 收到下单请求 amount=99
DEBUG create_order{user_id=42}:save_order{amount=99}: 开始写库
DEBUG create_order{user_id=42}:save_order{amount=99}: 写库完成
INFO create_order{user_id=42}: 下单完成 order_id=7001
```

注意 `create_order{user_id=42}:save_order{amount=99}`—— **span 是嵌套的**，  
下游函数的日志自动挂着上游的上下文。两条并发请求的日志再也不会混淆（各自的 user_id 钉在每一行上）。这就是「链路追踪」的最小形态；  
接上 OpenTelemetry 后，同样的 span 结构直接变成 Jaeger 里的调用树。

`#[instrument]` 的常用姿势：

- 默认把 **所有参数** 记为字段——参数太大/含敏感信息用 `skip(params)` 排除，再用 `fields(...)` 手工挑着放；
- ⚠️ 手动管理 span 时别用 `span.enter()` 跨 `.await`（guard 跨 await 的老问题，  
  [《共享状态：Arc / RwLock》](../async/shared-state.md) 同款）—— **async fn 一律用 `#[instrument]`**，  
  它以正确方式（instrument 到 Future 上）处理了这一切。

----

# 订阅端与 EnvFilter

> 订阅端决定日志去哪、什么格式、过滤规则。

```rust
tracing_subscriber::fmt()
    .with_env_filter(
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info,tracing_lab=debug".into()),  // 默认过滤规则
    )
    .with_target(true)      // 显示日志来源模块
    .init();                // 装为全局订阅端（进程内只能 init 一次）
```

EnvFilter 的规则语法（RUST_LOG 环境变量，生态通用约定）：

```bash
RUST_LOG=debug                          # 全局 debug
RUST_LOG=info,tracing_lab=debug         # 全局 info，本 crate debug
RUST_LOG=info,tower_http=warn           # 把 TraceLayer 的请求日志调安静
RUST_LOG=engineering_tracing::handlers=trace    # 精确到模块路径
```

按模块细分是 Go 常见日志库没有的能力——排障时把出问题的模块单独调到 trace，其他保持安静。其他常用配置：  
`.json()`（JSON 行输出，日志平台采集用）、`.with_level/with_line_number`、  
`.compact()`。生产标配组合：**本地 fmt 彩色、线上 json + 采集**。

----

# TraceLayer 自动 span

> 给每个请求自动创建 span，记录耗时/状态码—— [《中间件与优雅退出》](../http/middleware-shutdown.md) 手写的 log_requests 中间件的工业级替代。

```rust
let app = Router::new()
    .route("/order", get(create_order))
    .layer(TraceLayer::new_for_http());
```

它给每个请求自动创建 span（method/uri/version 为字段）、记录开始/结束/耗时/状态码。默认在 DEBUG 级别输出（`RUST_LOG=tower_http=debug` 可见），  
可 `.on_response(...)` 等定制。

----

# 去往生产的下一站

> 选读，几个延长线。

- **OpenTelemetry**：`tracing-opentelemetry` 把 span 导出为分布式追踪（Jaeger/Tempo），  
  代码里的 `#[instrument]` 一行不用改；
- **日志落盘/切割**：`tracing-appender`（非阻塞写文件 + 按天切割）；
- **动态改级别**：`tracing_subscriber::reload` 运行时调整过滤（配 [《通知与热更新》](../async/notify-watch.md) 的 watch 做配置热更新，  
  正好串上）。

----

# 动手实验

1. **过滤级别体感**：分别用默认、`RUST_LOG=debug`、`RUST_LOG=info,tower_http=debug` 跑，  
   同一个 curl 观察输出差异（第三种能看到 TraceLayer 的请求开始/结束行）；
2. **看 span 嵌套**：curl 一次 /order，找到 `create_order{...}:save_order{...}` 前缀——数一数每行日志自动携带了哪些不是你手写的字段；
3. **验证并发不串线**：两个终端同时 `curl "...user_id=1..."` 和 `...user_id=2...`（把 save_order 的 sleep 调大到 2 秒制造交错）——日志交错但每行的 user_id 清清楚楚；
4. **JSON 输出**：把 `.with_target(true)` 换成 `.json()`，重跑看每行变成一条可采集的 JSON；
5. **敏感字段**：给 OrderParams 加个 `password` 字段，观察 `#[instrument]` 默认会不会记录（会！  
  ）——用 skip 排除，体会为什么默认 skip 再手选是好习惯。

----

# 三句话带走

1. **event 记时刻、span 记区间**：结构化字段代替字符串拼接；span 是跟着 Future 走的请求上下文——Go 里靠 ctx 手动传的东西，  
   这里 `#[instrument]` 一个属性全自动。
2. **门面与订阅端分离**：代码只管 `info!(k = v, "msg")`，格式/去向/过滤由 `tracing_subscriber` 统一定；  
   `RUST_LOG` 支持按模块细分级别。
3. **HTTP 服务标配 `TraceLayer`**；async 函数的 span 一律 `#[instrument]`（别手动 enter 跨 await）；  
   上生产的延长线是 OpenTelemetry，代码零改动。

----

# 附：本课生词表

- **`info!` / `warn!` / …（事件宏）** ——五级结构化日志；字段 `k = v`，`?v` Debug 格式、  
  `%v` Display 格式。
- **span** ——带时间范围和字段的上下文；嵌套形成链路；范围内的 event 自动携带其字段。
- **`#[instrument]`** ——给函数包 span 的属性宏：参数自动成字段；`skip(...)` 排除、  
  `fields(...)` 手选；async fn 的正确姿势。
- **`tracing_subscriber::fmt()` / `.init()`** ——控制台格式订阅端及全局安装（进程内一次）；  
  `.json()` 切 JSON 输出。
- **`EnvFilter` / `RUST_LOG`** ——按「全局级别 + crate/模块=级别」过滤的规则串；生态通用。
- **`TraceLayer::new_for_http()`** ——tower-http 的请求跟踪层：每请求一个 span + 自动记录耗时/状态码；  
  手写中间件的工业版。
- **`target`** ——日志来源（默认是模块路径）；EnvFilter 按它过滤。
- **`tracing-appender` / `tracing-opentelemetry`（提及）** ——文件落盘切割 / 导出分布式追踪。
