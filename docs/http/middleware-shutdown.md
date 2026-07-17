# 中间件与优雅退出

> 代码：`code/http-middleware-shutdown/`　运行：`cargo run -p http-middleware-shutdown`

补上两个「生产环境必备」的能力：**请求日志中间件**（顺便理解洋葱模型）、 **优雅退出**（Ctrl-C 后等在途请求做完再退）。  
同时演示一个工程实践：**直接复用 [《从零手写 HTTP》](http-from-scratch.md) 的 lib crate**，  
不重复造轮子。

----

# 复用别的 crate

> lib + bin 布局的回报：本课直接拿 [《从零手写 HTTP》](http-from-scratch.md) 的 state/handler/updater 来用，  
> 不重复写。

本课 `Cargo.toml`：

```toml
[dependencies]
http-http-from-scratch = { path = "../http-http-from-scratch" }
```

于是 main.rs 直接拿它的东西用：

```rust
use http_http_from_scratch::handler;
use http_http_from_scratch::state::AppState;
use http_http_from_scratch::updater::run_updater;
```

要点：这就是「lib + bin」布局的回报——核心逻辑在 lib 里，别的 crate（或集成测试）直接依赖；  
`{ path = "..." }` 是 **路径依赖**（不从 crates.io 拉，用本地目录的 crate，  
同工作区内共享依赖版本）。对照 Go：≈ 把它做成 package，这里 import 复用；连「导出规则」都对应：  
Rust 的 `pub` ≈ Go 首字母大写。

----

# 中间件洋葱模型

> 中间件 = 在请求到达 handler 前后各插一段逻辑。请求从外层一层层穿进去，响应一层层穿出来。

```
请求 → [日志] → [鉴权] → [限流] → handler
响应 ← [日志] ← [鉴权] ← [限流] ←   ↓
```

日志、鉴权、限流、CORS 都是一层「皮」。

## from_fn 把函数变中间件

```rust
async fn log_requests(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    let res = next.run(req).await;  // ← 交给内层（后续中间件/handler）

    // next.run 返回后 = 响应往外穿的阶段，可以读/改响应
    logln!("[req] {method} {uri} -> {} ({:?})", res.status(), start.elapsed());
    res
}
```

三个位置的含义：

| 位置 | 阶段 | 能做什么 |
| --- | --- | --- |
| `next.run` 之前 | 前置处理 | 读请求、记时、鉴权 |
| `next.run(req).await` | 交棒给内层 | —— |
| `next.run` 之后 | 后置处理 | 读/改响应、记日志 |

前置阶段直接 `return` 一个响应而 **不调用 next.run** = 拦截（鉴权失败直接挡回去）。挂到路由上：

```rust
let app = Router::new()
    .route("/data", get(handler::get_data))
    .layer(middleware::from_fn(log_requests))  // 全局中间件
    .with_state(state);
```

对照 Gin：

```go
func Logger() gin.HandlerFunc {
    return func(c *gin.Context) {
        start := time.Now()
        c.Next()   // ← ≈ next.run(req).await
        log.Printf("%s %s -> %d (%v)", ...)
    }
}
r.Use(Logger())
```

| axum | Gin |
| --- | --- |
| `next.run(req).await` | `c.Next()` |
| 不调用 next.run 直接返回 | `c.Abort()` |
| `.layer(...)` | `r.Use(...)` |

## 层的顺序与 tower 生态

多个 `.layer()` 的规则：**后加的在外层**；`.layer(A).layer(B)` → 请求先过 B、  
再过 A、再到 handler；要精确控制顺序（先限流→再鉴权→再日志），用 `tower::ServiceBuilder` 组合，  
顺序更直观。

`.layer()` 接受任何 tower `Layer`（回忆 [《从零手写 HTTP》](http-from-scratch.md)：  
axum 站在 tower 上），所以能直接用整个 **tower / tower-http 生态**：超时（TimeoutLayer）、  
限流、压缩、CORS、请求体大小限制…全是现成的。

> **生产建议**：真实项目常用 `tower_http::trace::TraceLayer` + `tracing` 做 **结构化日志**（带 span、  
> 级别、字段、可接 OpenTelemetry），见 [《tracing 结构化日志》](../engineering/tracing.md)。  
> 本课手写 from_fn 是为了看清中间件 **本质**；看懂后换 TraceLayer 只是一行 `.layer(...)`。

----

# 优雅退出

> 「优雅退出」= 收到停止信号后：不再接受新连接 → 把已在处理的请求 **做完** → 再退出进程。

不做会怎样：正在传输的响应被硬掐断，客户端看到连接重置；请求写数据库写到一半，留下不一致；k8s 滚动更新/容器重启时， **每次发版抖一批 5xx**。

怎么写：

```rust
axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())  // 传入"关闭信号 Future"
    .await
    .unwrap();

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("无法安装 Ctrl-C 处理器");
    logln!("收到 Ctrl-C：停止接收新连接，等待在途请求完成...");
}
```

原理：`with_graceful_shutdown(fut)`——fut 一完成 → 触发优雅关闭（停 accept → 等在途请求 → serve 返回）；  
回忆 [《async 基础》](../async/basics.md)：Future 是惰性的——`shutdown_signal()` 传进去时没有执行，  
是 serve 内部拿它和「处理请求」一起 select，谁先好谁生效。

**生产要点：也监听 SIGTERM。** 容器/k8s 停 Pod 发的是 **SIGTERM**（不是 SIGINT），生产通常两个都等：

```rust
async fn shutdown_signal() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.unwrap(); };

    #[cfg(unix)]
    let term = async {
        tokio::signal::unix::signal(
            tokio::signal::unix::SignalKind::terminate(),
        ).unwrap().recv().await;
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();  // 非 unix：永不触发的占位

    tokio::select! {
        _ = ctrl_c => {},
        _ = term => {},
    }
}
```

`select!` 等两个信号，任一到达即关闭；生产还常配 **兜底超时**（在途请求最多等 N 秒，超时强退，防止卡死的请求让进程永不退出）。对照 Go：

```go
quit := make(chan os.Signal, 1)
signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
<-quit
ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
defer cancel()
srv.Shutdown(ctx)
```

`with_graceful_shutdown` 把这套模板收敛成一行 + 一个信号 Future。

----

# 跑起来观察

```bash
cargo run -p http-middleware-shutdown
```

- 访问几次接口 → 终端打印 `方法 路径 -> 状态码 (耗时)`（中间件工作中）；
- 按 Ctrl-C → 先打印「等待在途请求完成」，再打印「已优雅退出」。

看清「优雅」的效果：给某 handler 加 `sleep(5s).await` 模拟慢请求；curl 它，5 秒内按 Ctrl-C；  
观察：服务 **没有立刻退出**，等这条 curl 拿到完整响应才退。

----

# 动手实验

1. **前置拦截**：改造 log_requests，`uri.path() == "/admin"` 时不调用 next.run，  
   直接 `return (StatusCode::FORBIDDEN, "no").into_response();`——验证「中间件拦截」= Gin 的 c.Abort()；
2. **验证优雅**：按上面「看清效果」的三步做一遍；
3. **换真中间件**（选做，引新依赖）：加 `tower-http`（trace）+ `tracing-subscriber`，  
   main 开头 `tracing_subscriber::fmt::init();`，把 from_fn 换成 `TraceLayer::new_for_http()`，  
   对比结构化日志。

----

# 三句话带走

1. **lib + bin 布局让核心逻辑可被复用**（本课直接 use 了手写 HTTP 那课），工作区内路径依赖零成本。
2. **中间件 = from_fn**：`next.run(req).await` 就是 c.Next()，前后即前置/后置，  
   不调用即拦截；`.layer()` 就是 r.Use()，且能接入整个 tower 生态。
3. **`with_graceful_shutdown(signal)`** = Go「等信号 + srv.Shutdown」的一行版；  
   生产同时等 SIGINT/SIGTERM 并加兜底超时。

----

至此，你已从 async 语法走到「可上生产的 HTTP 服务骨架」🎉 [《接入 Redis》](redis.md) 把数据从进程内存搬到 **Redis**：  
跨实例共享、可持久。

----

# 附：本课生词表

> 通用语法见 [《Rust 语法底座》](../start/syntax-primer.md)；Router/State 见 [《从零手写 HTTP》](http-from-scratch.md) 生词表。

- **`http-http-from-scratch = { path = "../http-http-from-scratch" }`（路径依赖）** ——依赖声明的另一种形态：  
  用本地目录的 crate，不从 crates.io 拉；工作区内互相引用全靠它；≈ Go 同 module 内 import 内部 package，  
  或 go.mod replace 指本地。
- **`axum::extract::Request` / `axum::response::Response`** ——原始的 HTTP 请求/响应类型（未经提取器加工）；  
  中间件工作在这一层：拿整个 Request，产出整个 Response。
- **`middleware::from_fn(f)`** ——把签名为 `async fn(Request, Next) -> Response` 的普通函数变成中间件层；  
  写自定义中间件最简单的方式。
- **`Next` / `next.run(req).await`** ——Next 代表「洋葱的内层」（后续中间件 + handler）；  
  `next.run(req)` 把请求交进去、拿响应回来 ≈ `c.Next()`；不调用它直接 return 响应 = 拦截 ≈ `c.Abort()`。
- **`req.method().clone()`（为什么要 clone）** ——`next.run(req)` 会把 req 的 **所有权** move 走，  
  之后你不能再碰 req；想在响应阶段还打印方法/路径，就得在交出去 **之前** 复制留底——所有权规则在中间件里的直观体现。
- **`.layer(...)`** ——给整棵路由套一层中间件 ≈ `r.Use(...)`；可多次调用， **后加的在外层**；  
  精确控制顺序用 `tower::ServiceBuilder`。
- **`tokio::signal::ctrl_c()`** ——返回「收到 Ctrl-C（SIGINT）时完成」的 Future；  
  ≈ Go 的 signal.Notify + `<-quit` 二合一。
- **`.with_graceful_shutdown(fut)`** ——给 serve 挂「关闭信号 Future」：  
  fut 完成 → 停接新连接 → 等在途请求 → serve 返回；≈ `srv.Shutdown(ctx)`，  
  声明式一行。
- **`#[cfg(unix)]`** ——条件编译：这段代码只在 unix 目标上编译（Windows 上不存在）；  
  ≈ Go 的 `//go:build unix`；SIGTERM 只有 unix 有。
- **`std::future::pending::<()>()`** ——一个 **永远不会完成** 的 Future；  
  用途：在 select! 里当「永不触发」的占位；`::<()>` 是 turbofish 指定 Output 类型。
