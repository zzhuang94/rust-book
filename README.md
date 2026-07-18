# Rust 手册

这是一套写给 **已经会用 Go（尤其写过 Gin 服务）、想系统进入 Rust 的后端开发者** 的教程。  
它不追求铺满 Rust 的每一条标准细节，  
而是围绕一条主线把最关键的一圈能力讲透：

> 从 Rust 异步基础，一路走到能写出一个「**定期更新内存数据 + 高并发读取**」的生产级 HTTP 服务。

全书 **文档为主、代码为辅**：每个概念都尽量给「你在 Go 里怎么写 → Rust 里怎么写」的并排对照，配可独立运行的多场景示例；  
所有示例代码都在 [`code/`](code/) 这个 Cargo 工作空间里，可直接跑。

----

# 这套文档适合谁

- 你主要写 **Go**，用过 Gin/net-http 写过完整的 HTTP 服务，熟悉 goroutine、channel、`context`。
- 你对 Rust 的所有权、`async`、生命周期只有模糊印象，想要一条不绕弯的上手路径。
- 你更关心「真实服务端项目里怎么写、怎么排错」，而不是先啃语言规范。

**不面向零基础读者**：如果你还不熟悉分支/循环/函数/并发的基本概念，建议先补一门语言的服务端实践，再回来。

----

# 全书怎么组织

按主题分成九组，由浅入深；括号里是该组解决的问题。

| 分组 | 解决什么 | 关键章节 |
| --- | --- | --- |
| **开始** | 装好环境、建立 Go→Rust 心智地图 | [《环境与工具链》](docs/start/toolchain.md)、 [《Go → Rust 语言对照》](docs/start/go-vs-rust.md)、 [《Rust 语法底座》](docs/start/syntax-primer.md) |
| **语言地基** | 从基础类型到所有权、集合、智能指针、错误处理 | [《基础类型》](docs/lang/basics.md) → [《流程控制》](docs/lang/control-flow.md) → [《所有权与借用》](docs/lang/ownership.md) → [《函数与闭包》](docs/lang/functions-closures.md) → [《生命周期》](docs/lang/lifetimes.md) → …（完整列表见侧栏） |
| **操作系统** | 组成原理、CPU/内存/磁盘、进程线程、调度、fd、阻塞与 epoll 等硬实力地基 | [《计算机组成入门》](docs/os/computer-basics.md) → [《进程与线程》](docs/os/process-thread.md) → [《阻塞与 IO 多路复用》](docs/os/blocking-io.md) → …（完整列表见侧栏） |
| **并发基础** | Rust 线程模型 + Go GMP 参照系 | [《Rust 多线程与并发》](docs/concurrency/threads.md)、 [《Go 并发实现（GMP）》](docs/concurrency/go-gmp.md) |
| **网络编程** | 从分层、Socket、TCP/UDP 到 HTTP/TLS/长连接/RPC，再到排障与进阶 | [《网络分层与模型》](docs/network/layers.md) → [《Socket 详解》](docs/network/socket.md) → [《TCP 保姆级》](docs/network/tcp.md) → [《HTTP 协议入门》](docs/network/http-protocol.md) → …（完整列表见侧栏） |
| **异步主线** | async/await、Tokio、共享状态、并发工具箱 | [《async 基础》](docs/async/basics.md)、 [《Tokio 运行时》](docs/async/tokio.md)、 [《共享状态：Arc / RwLock》](docs/async/shared-state.md)、 [《超时、限流与任务组》](docs/async/task-control.md)、 [《通知与热更新》](docs/async/notify-watch.md)、 [《生产任务生命周期》](docs/async/service-lifecycle.md) |
| **HTTP 服务** | 从裸 TCP 手写 HTTP 到 axum，再到请求上游、数据库和缓存 | [《从零手写 HTTP》](docs/http/http-from-scratch.md)、 [《axum 入门》](docs/http/axum.md)、 [《JSON 序列化与反序列化》](docs/http/serde-json.md)、 [《读写接口与错误处理》](docs/http/rest.md)、 [《ArcSwap 无锁读》](docs/http/arcswap.md)、 [《中间件与优雅退出》](docs/http/middleware-shutdown.md)、 [《reqwest 与上游容错》](docs/http/reqwest-resilience.md)、 [《接入 Redis》](docs/http/redis.md)、 [《Redis Cluster》](docs/http/redis-cluster.md)、 [《sqlx 数据库》](docs/http/sqlx.md) |
| **工程实践** | 把语言能力接到真实项目：规范、解耦、测试、日志、依赖、部署 | [《代码规范与最佳实践》](docs/engineering/code-quality.md)、 [《测试》](docs/engineering/testing.md)、 [《网络集成测试》](docs/engineering/network-testing.md)、 [《tracing 结构化日志》](docs/engineering/tracing.md)、 [《Cargo 生态与依赖管理》](docs/engineering/cargo-ecosystem.md)、 [《构建与部署》](docs/engineering/build-deploy.md) |
| **附录** | 写码时随手查 | [《Go → Rust 翻译词典》](docs/appendix/go-rust-dict.md) |

**文档风格**：每课正文讲概念与原理，穿插 **🔬 底层视角**（把结论钉到操作系统事实上）；末尾都有 **「附：  
本课生词表」**，按出现顺序解释该课代码里的每个新面孔。写码时「这个 Go 里怎么写来着」→ 直接查 [《Go → Rust 翻译词典》](docs/appendix/go-rust-dict.md)。

----

# 推荐阅读顺序

- **想尽快跑起服务**：开始 → 语言地基（重点啃 [《所有权与借用》](docs/lang/ownership.md) +  
  [《生命周期》](docs/lang/lifetimes.md)）→ 操作系统 → 并发基础 → 网络编程 → 异步主线 → HTTP 服务，工程实践按需补。
- **对机器如何工作几乎没概念**：先按侧栏「操作系统」读完组成、CPU/内存、进程线程、  
  用户态、阻塞 IO，再进并发与网络。
- **网络是黑盒、只会写业务接口**：操作系统之后，按侧栏「网络编程」从上到下读；  
  至少读完分层、寻址、Socket、TCP、HTTP 协议、超时重试，再进 HTTP 服务组。
- **想稳扎稳打**：按侧边栏从上到下通读；所有权两章是全书承重墙，值得花最多时间。
- **已在改真实 Rust 项目**：先读一章 → 去对应 `code/<目录-文件名>/` 找示例 → 再回来对照。

## 示例代码怎么找

约定：**一章一个 Cargo 项目**，目录名 = `docs` 子目录名 + `-` + 文件名（无 `.md`）。

| 文档 | 示例目录 | 运行（先 `cd code`） |
| --- | --- | --- |
| `docs/lang/basics.md` | `code/lang-basics/` | `cargo run -p lang-basics` |
| `docs/lang/ownership.md` | `code/lang-ownership/` | `cargo run -p lang-ownership` |
| `docs/http/serde-json.md` | `code/http-serde-json/` | `cargo run -p http-serde-json` |

每篇文档开头的引用块都会再次标明路径与运行命令。全部示例均使用
“文档目录名 + 文件名”的统一命名。

----

# 怎么运行示例代码

代码在 [`code/`](code/) 子目录（一个独立的 Cargo 工作空间）。先进入它：

```bash
cd code
# —— 语言地基（一章一 crate，目录 = docs 子目录-文件名）——
cargo run -p lang-basics
cargo run -p lang-ownership
cargo run -p lang-functions-closures
cargo run -p concurrency-threads
# —— 操作系统（一章一 crate）——
cargo run -p os-computer-basics
cargo run -p os-cpu-memory
cargo run -p os-disk-io
cargo run -p os-process-thread
cargo run -p os-user-kernel
cargo run -p os-scheduling
cargo run -p os-coroutine-state
cargo run -p os-virtual-memory
cargo run -p os-file-fd
cargo run -p os-blocking-io
cargo run -p os-sync-primitives
cargo run -p os-perf-cost
cargo run -p os-signals-lifecycle
cargo run -p os-time-clock
cargo run -p os-cgroup-container
# —— 网络编程（一章一 crate）——
cargo run -p network-layers            # 分层与地址直觉
cargo run -p network-addressing        # IP / 端口 / DNS
cargo run -p network-socket            # bind / listen / accept / connect
cargo run -p network-tcp               # 粘包与按行划界
cargo run -p network-udp-sockets       # UDP + IPv4/IPv6 双栈 socket
cargo run -p network-http-protocol     # 裸看 HTTP 原文
cargo run -p network-tls               # 本地自签 TLS echo
cargo run -p network-websocket         # WebSocket echo
cargo run -p network-mqtt              # MQTT（需本机 :1883 broker，否则友好退出）
cargo run -p network-rpc-grpc          # gRPC helloworld（本机 :50051）
cargo run -p network-timeouts-retries  # 超时与重试
cargo run -p network-debug-tools       # 本地端口供 ss/netstat 观察
cargo run -p network-load-balancing    # 简易轮询转发
cargo run -p network-proxy-nat         # 极简反向代理
cargo run -p network-quic-http3        # UDP 边界 vs TCP；可选 --features quic-demo
# —— 异步 / HTTP（一章一 crate，同样使用目录-文件名）——
cargo run -p async-basics     # async 基础
cargo run -p async-tokio    # Tokio 运行时
cargo run -p async-shared-state     # 共享状态
cargo run -p async-task-control     # 超时/限流/任务组
cargo run -p async-notify-watch     # watch/broadcast/Notify/OnceCell
cargo run -p async-service-lifecycle # 任务监管、取消与限时退出
cargo run -p http-http-from-scratch --bin step1_single_thread   # 手写 HTTP 第0步：纯标准库
cargo run -p http-http-from-scratch --bin step2_thread_per_conn # 第1步：每连接一线程
cargo run -p http-http-from-scratch --bin step3_tokio_task      # 第2步：每连接一个 tokio 任务
cargo run -p http-http-from-scratch      # 手写 HTTP 终点：axum 版，监听 :7080
cargo run -p http-serde-json  # JSON 序列化（不启端口）
cargo run -p http-rest        # 读写接口 + 路径参数 + 错误处理
cargo run -p http-arcswap   # ArcSwap 无锁读版
cargo run -p http-middleware-shutdown  # 中间件 + 优雅退出
cargo run -p http-reqwest-resilience # HTTP 客户端、超时与旧快照降级
cargo run -p http-redis     # 接入 Redis（需先启动 Redis）
cargo run -p http-redis-cluster # Redis Cluster（需先准备本地集群）
cargo test -p engineering-testing     # 测试（注意是 test 不是 run）
cargo test -p engineering-network-testing # 真实网络与故障注入测试
cargo run -p engineering-tracing      # 结构化日志（试试 RUST_LOG=debug 前缀）
cargo run -p http-sqlx         # 需先起 Postgres
```

> HTTP 系列都监听 `:7080`，一次只跑一个。Redis 课还需要：`docker run --rm -p 6379:6379 redis`。

服务跑起来后，另开一个终端：

```bash
curl http://127.0.0.1:7080/          # ok
curl http://127.0.0.1:7080/data      # 当前内存快照（每 3 秒变一次）
curl http://127.0.0.1:7080/stats     # 版本号、条目数、累计读取次数
```

## 关于输出的时间戳（logln!）

为方便观察异步时序，各课示例统一用 **`logln!`** 代替 `println!`：用法完全一样，只是每行自动加毫秒时间戳前缀：

```
[2026-07-11 22:22:01.123] say(A, 100) 准备开始执行
```

它定义在通用工具 crate [`labkit`](code/labkit/src/lib.rs) 里（内部用 `chrono` 格式化本地时间，  
对照 Go 的 `time.Now().Format("2006-01-02 15:04:05.000")`）。  
文档正文里为聚焦主题仍写 `println!`， **实际 `.rs` 代码用的是 `logln!`**，二者只差一个时间戳前缀，  
阅读时可等同看待。

----

# 一张「Go → Rust 异步」心智对照表

| 你在 Go 里怎么做 | Rust / Tokio 里对应什么 |
| --- | --- |
| 函数天生同步阻塞 | `async fn` 返回一个惰性的 `Future`，`.await` 才推进 |
| `go f()` 开 goroutine | `tokio::spawn(async { ... })` |
| `time.Sleep` | `tokio::time::sleep(...).await` |
| `time.NewTicker` | `tokio::time::interval(...)` |
| `ch := make(chan T, n)` | `let (tx, rx) = tokio::sync::mpsc::channel::<T>(n)` |
| `select { case ... }` | `tokio::select! { ... }` |
| `sync.WaitGroup` | `join!` / 收集 `JoinHandle` 再逐个 `.await` |
| `sync.RWMutex` | `std::sync::RwLock`（读写都很短时） |
| `sync.Mutex` | `std::sync::Mutex` / `tokio::sync::Mutex`（跨 await 时用后者） |
| `atomic.Int64` | `std::sync::atomic::AtomicU64` |
| 共享指针（GC 管理） | `Arc<T>`（引用计数，无 GC） |
| Gin 的 `*gin.Context` | axum 的「提取器」参数 + 返回值自动转响应 |
| `r.GET("/x", h)` | `Router::new().route("/x", get(h))` |
| `r.Run(":7080")` | `axum::serve(listener, app).await` |

# 一句话说清 Rust 异步和 Go 的根本差异

- **Go**：goroutine 是「绿色线程」，运行时（GMP 调度器）默认就在，`go` 一下就并发，Sleep/IO 会自动让出。并发是语言内建的。
- **Rust**：语言只提供 `async/await` 语法和 `Future` 这个「可暂停的状态机」，但 **不自带运行时**。  
  你需要一个执行器（executor）去真正跑这些 Future —— 这就是 **Tokio** 的角色。可以理解成：  
  **Tokio ≈ 你手动引入的、可插拔的 Go 调度器**。

----

# 本地预览与部署

本书用 [docsify](https://docsify.js.org) 渲染，纯静态、无需构建：

```bash
# 本地预览（任选其一）
docsify serve .
# 或任意静态服务器
python -m http.server 3000
```

推 GitHub Pages：仓库已含 `.nojekyll`，把本目录作为 Pages 根发布即可（客户端渲染，无需 Jekyll 构建）。
