# 从 TCP 造到 axum

> 代码：`code/http-http-from-scratch/`。本课有 4 个可运行程序，对应四步进阶（都监听 :7080，一次跑一个）：
>
> | 步 | 运行命令 | 内容 |
> | --- | --- | --- |
> | 第 0 步 | `cargo run -p http-http-from-scratch --bin step1_single_thread` | 纯标准库手写 HTTP（单线程阻塞） |
> | 第 1 步 | `cargo run -p http-http-from-scratch --bin step2_thread_per_conn` | 每连接一个线程（≈ Go net/http 模型） |
> | 第 2 步 | `cargo run -p http-http-from-scratch --bin step3_tokio_task` | 线程换成 tokio 任务（异步） |
> | 第 3 步 | `cargo run -p http-http-from-scratch` | hyper + axum 正式版（本课终点） |

在 Gin 里你写的第一行 Web 代码就是 `r.GET(...)`——框架把底下的一切都藏好了。这一课反过来：  
**先不用任何框架，从 TCP 和 HTTP 协议本身开始，亲手造一个服务器**，每一步解决上一步的一个真问题，  
最后你会清楚地知道 tokio、hyper、axum 各自到底替你干了什么。

----

# HTTP 是什么

> 一句话：**HTTP 就是跑在 TCP 连接上的「一问一答」纯文本协议**。没有魔法，就是格式固定的字符串。

浏览器/curl 发给服务器的「问」（请求）长这样——每行以 `\r\n`（回车+换行）结尾：

```
GET /slow HTTP/1.1\r\n          ← 请求行：方法 路径 协议版本
Host: 127.0.0.1:7080\r\n        ← 请求头（header），每行一个键值对
User-Agent: curl/8.5.0\r\n
Accept: */*\r\n
\r\n                            ← 空行 = 头结束（POST 的话后面还跟着 body）
```

服务器写回去的「答」（响应）和它对称：

```
HTTP/1.1 200 OK\r\n                            ← 状态行：版本 状态码 原因短语
Content-Type: text/plain; charset=utf-8\r\n    ← 响应头
Content-Length: 33\r\n                          ← body 的字节数（对方靠它知道读多少）
\r\n                                            ← 空行 = 头结束
hello，这是一份手写的 HTTP 响应                   ← body
```

三个此后处处会用到的要点：

- **`\r\n` 是 HTTP 规定的行分隔符**（不是 Unix 的 `\n`），空行 `\r\n\r\n` 标志「头部结束」；
- **`Content-Length` 是 body 的字节数** ——一个汉字 UTF-8 占 3 字节。Rust 的 `String::len()` 恰好返回字节数，  
  直接能用；
- 所以「写一个 HTTP 服务器」的最小任务清单是：**收 TCP 连接 → 读一段文本 → 从第一行抠出方法和路径 → 拼一段格式正确的文本写回去**。  
  就这么多。

想亲眼看协议原文？跑起任何一步的服务器后：`curl -v http://127.0.0.1:7080/`——`-v` 会把请求（`>` 开头）和响应（`<` 开头）的每一行都打给你。

> 🔬 **底层视角：一条「连接」在操作系统里是什么**（见 [《操作系统基础》](../concurrency/os-basics.md)）：  
> 程序向内核要一个 socket，拿到的是一个 **文件描述符**（fd，就是个小整数），背后是内核替这条连接维护的一对缓冲区——对方发来的字节先堆在 **接收缓冲区**，  
> 你 `read` 只是把它们拷进自己的 buf；你 `write` 也只是拷进 **发送缓冲区**，内核再慢慢经网卡发出去。  
> 所以「读没数据的 socket 会阻塞」的确切含义是：接收缓冲区是空的，内核把你的线程挂成阻塞态，等网卡送来新字节再叫醒你。

> 对照 Go：这层知识你在 Go 里也从没手写过，因为 `net/http` 全包了。掀开地毯看一次地基，框架就不再神秘。

----

# 单线程阻塞版

> 第 0 步：纯标准库手写最小 HTTP 服务器。骨架就四件事，但有个致命缺陷。

代码：[step1_single_thread.rs](../../code/http-http-from-scratch/src/bin/step1_single_thread.rs)。

```rust
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn main() {
    // (1) 监听端口 ≈ Go 的 net.Listen("tcp", ":7080")
    let listener = TcpListener::bind("0.0.0.0:7080").unwrap();

    // (2) 逐个接收连接 ≈ Go 的 for { conn, _ := ln.Accept() }
    for stream in listener.incoming() {
        handle(stream.unwrap());   // (3) ★ 注意：普通函数调用 —— 处理完才接下一个！
    }
}

fn handle(mut stream: TcpStream) {
    // (4) 读原始字节 → 按上一节的格式解析和回写
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).unwrap_or(0);
    let request = String::from_utf8_lossy(&buf[..n]);

    // 从请求行 "GET /slow HTTP/1.1" 抠出路径（按空格分割取第 2 段）
    let path = request.lines().next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/").to_string();

    // 手工"路由 + handler"
    let (status, body) = match path.as_str() {
        "/" => ("200 OK", "hello".to_string()),
        "/slow" => { std::thread::sleep(std::time::Duration::from_secs(5)); ("200 OK", "慢请求完成".into()) }
        _ => ("404 Not Found", format!("没有这个路径: {path}")),
    };

    // 手工拼响应文本，写回 TCP
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).unwrap();
}
```

几个逐行的「为什么」：

- `TcpListener::bind` / `incoming()`：标准库的 **同步阻塞** TCP。`incoming()` 是个迭代器，  
  每来一个连接产出一个 `TcpStream`（≈ Go 的 `net.Conn`）；
- `stream.read(&mut buf)`：把对方发来的字节读进缓冲区， **没数据就干等**（阻塞）。返回读到的字节数 n；
- `String::from_utf8_lossy(&buf[..n])`：把字节切片按 UTF-8 转成字符串，  
  遇到非法字节用 � 替代（「lossy」=有损但绝不失败）；
- 我们回了 `Connection: close`：应答完就关连接（函数结尾 stream 被 drop = 关闭），省去 keep-alive 的复杂度。

**跑起来做那个关键实验**：

```bash
cargo run -p http-http-from-scratch --bin step1_single_thread
# 终端A
curl http://127.0.0.1:7080/slow     # 会等 5 秒
# 趁它没返回，立刻在终端B
curl http://127.0.0.1:7080/         # ★ 也被卡住 5 秒！
```

原因就在代码第 (3) 处：`handle(stream)` 是普通调用，处理完当前连接才回去 accept 下一个。  
**一个慢请求，卡死整个服务器** ——这就是第 0 步的致命缺陷，也是下一步存在的理由。

> 🔬 **底层视角：被「卡住」的终端 B 其实排在内核的队列里**：服务器忙着 /slow 时没人调 accept，  
> 但终端 B 的 TCP 三次握手仍然由 **内核** 默默完成了——新连接被放进这个监听 socket 的 **accept 队列**（长度叫 backlog，  
> 通常几百）排队等待。等这个队列也塞满，再来的连接才会真正超时/被拒——这就是单线程服务器在压力下的完整死法：先排队，  
> 后拒客。

----

# 每连接一线程

> 第 1 步：每连接开一个线程处理。这正是 Go net/http 的模型——但 OS 线程不是免费的。

代码：[step2_thread_per_conn.rs](../../code/http-http-from-scratch/src/bin/step2_thread_per_conn.rs)。  
相对第 0 步只改了一行：

```rust
for stream in listener.incoming() {
    let stream = stream.unwrap();
    thread::spawn(move || handle(stream));   // ★ 开线程处理，主循环立刻回去 accept
}
```

重跑上一步的实验：`/slow` 在自己的线程里睡， **`/` 立即返回了**。日志里还能看到不同的线程 ID。

> 对照 Go：`go handleConn(conn)`—— **`net/http` 内部就是每连接一个 goroutine**。  
> 你写 Gin 时享受的并发，本质就是这一行。

但 OS 线程不是免费的：

| | OS 线程（本步） | goroutine（Go） | 异步任务（下一步） |
| --- | --- | --- | --- |
| 初始栈 | ~8MB 虚拟内存 | ~2KB，可增长 | 几百字节的状态机 |
| 创建/切换 | 过内核，微秒级 | 用户态，纳秒级 | 用户态，纳秒级 |
| 一万并发连接 | 一万个线程，调度器哭泣 | 轻松 | 轻松 |

「一万个并发连接怎么办」就是著名的 **C10K 问题**。Go 的答案是把线程做轻（goroutine）；Rust 的答案是异步任务——正好是你前三课学的全部东西。  
（线程为什么贵——栈、上下文切换、内核调度——完整背景见 [《操作系统基础》](../concurrency/os-basics.md)；  
纯线程路线的正确用法见 [《Rust 多线程与并发》](../concurrency/threads.md)。）

> 🔬 **底层视角：这个模型里，内核调度器本身就是「多路复用器」**：一万个线程各自阻塞在自己那条连接的 read 上，  
> 等于把「盯着一万个 socket」的活全部外包给了内核——哪个 socket 来了数据，内核就唤醒哪个线程。功能上完全正确，  
> 贵在 **计价单位**：每次唤醒都是一次完整的线程上下文切换（微秒级 + 缓存变冷），且一万份 8MB 的栈先押在那里。  
> 下一步的思路就是换个便宜的计价单位：还是让内核盯 socket（换成 epoll 批量盯），但被唤醒的是几百字节的任务，  
> 「切换」是纳秒级的函数调用。

----

# 换成 tokio 任务

> 第 2 步：线程换成 tokio 任务。结构和上一步完全一样，只有三处「同步→异步」的替换。

代码：[step3_tokio_task.rs](../../code/http-http-from-scratch/src/bin/step3_tokio_task.rs)。

| step2（线程版） | step3（tokio 版） |
| --- | --- |
| `std::net::TcpListener` | `tokio::net::TcpListener`（accept 是 async） |
| `thread::spawn(move \|\| handle(stream))` | `tokio::spawn(async move { handle(stream).await })` |
| `Read/Write` + `thread::sleep` | `AsyncReadExt/AsyncWriteExt` + `tokio::time::sleep` |

```rust
#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("0.0.0.0:7080").await.unwrap();
    loop {
        let (stream, addr) = listener.accept().await.unwrap();  // 等连接时让出线程
        tokio::spawn(async move { handle(stream).await });      // 每连接一个任务
    }
}
```

几个值得停下来的点：

- `stream.read(&mut buf).await`：数据没到就 **让出线程** 去伺候别的连接（step2 里是线程干等）。  
  要 `use tokio::io::{AsyncReadExt, AsyncWriteExt};` 才有这些方法——又是「trait 方法要 use 进来」（和 [《接入 Redis》](redis.md) 的 AsyncCommands 一个道理）；
- `/slow` 里必须用 `tokio::time::sleep(...).await`：这里要是手滑写了 `std::thread::sleep`，  
  就会卡住工作线程、重现 [《async 基础》](../async/basics.md) 场景(5) 的惨案——在真实服务器代码里体会那条铁律；
- 效果：一万并发连接 = 一万个几百字节的状态机，躺在「CPU 核数」个工作线程上被调度。 **你已经用前三课的知识亲手造出了一个异步 HTTP 服务器。**

> 🔬 **底层视角：`read().await` 底下发生了什么**：tokio 的 TcpStream 是 **非阻塞** 模式的 socket——接收缓冲区没数据时，  
> 内核不再挂起线程，而是立刻回一句「暂时没有」（EWOULDBLOCK）。tokio 收到这个回答后做三件事：把该 socket 登记进 epoll、  
> 给它挂上本任务的 Waker、让 poll 返回 Pending——工作线程转身去跑别的任务。等网卡送来数据，内核在下一次 epoll 查询里报告这个 socket，  
> tokio 调 waker，任务被重新排队、从暂停点继续。 [《async 基础》](../async/basics.md) 讲的那条 Waker 唤醒链，  
> 在这里第一次接上了真实的网卡。

----

# 玩具版还缺什么

> step3 能跑，但离生产差一整个协议栈。数一数它糊弄过去的脏活，就知道 hyper/axum 各管什么。

它糊弄过去的脏活：

- **读请求**：假设一次 `read()` 能读完整个请求（大请求会分多个 TCP 包到达）；没处理 POST body、  
  `Content-Length`/分块传输、URL 转义、超大请求防护……
- **连接管理**：没有 keep-alive（每请求一条新 TCP）、没有 HTTP/2 多路复用、没有 TLS；
- **路由**：`match path` 撑不了几个接口——没有路径参数（`/item/{id}`）、方法区分、中间件；
- **工程化**：没有类型化的参数提取、JSON 序列化、统一错误处理、优雅退出……

这些活分了两层，各有专业选手：

```
你的业务 handler
────────────────────────────
axum   ← 路由、参数提取、响应转换、中间件（≈ Gin 的位置）
tower  ← 通用 Service/Layer 抽象（中间件生态）
hyper  ← HTTP 协议本身：解析、keep-alive、HTTP/1.1 + HTTP/2（≈ net/http 的位置）
────────────────────────────
tokio  ← 异步运行时 + TCP（我们 step3 停在这层）
```

`tower::Service` 的本质一句话——「一个把 Request 异步变成 Response 的东西」：

```rust
// 概念简化
trait Service<Request> {
    type Response;
    async fn call(&mut self, req: Request) -> Self::Response;
}
```

你的整个 axum 应用（Router）最终就是一个大 Service；中间件（Layer）是包在外面的一层 Service，  
可层层嵌套（[《中间件与优雅退出》](middleware-shutdown.md) 的洋葱模型就从这来）。知道这个分层，  
报错里冒出 `Service`/`Layer`/`IntoResponse` 时你就知道它们是谁了。

> 对照 Gin：Gin 是「大而全」（路由中间件绑定渲染都自己来，底下是 net/http）；axum 是薄薄一层，  
> 把 HTTP 交给 hyper、把中间件抽象交给 tower——所以任何 tower 中间件（超时/限流/压缩/CORS）都能直接用。

----

# axum 正式版

> 第 3 步：让专业选手接管。本节代码就是 `cargo run -p http-http-from-scratch` 跑的正式版——后台任务每 3 秒刷新内存数据，  
> HTTP 接口高并发读取（[《共享状态：Arc / RwLock》](../async/shared-state.md) 模型 + 网络层）。

> 本节只讲「本项目用到的」那部分 axum。axum 的完整用法字典（Router 的 nest/merge/fallback、  
> 提取器和响应的全家桶、Gin↔axum 速查大表、handler 报错排查）单独成篇—— [《axum 入门》](axum.md)，  
> 建议本节读完接着读它。

## 项目结构 lib 多模块

```
http-http-from-scratch/
├── Cargo.toml
└── src/
    ├── bin/            ← 本课的第 0~2 步（三个独立可跑的小程序）
    ├── main.rs         启动入口：组装 + 启动（≈ Gin 的 main）
    ├── lib.rs          模块清单（pub mod ...）
    ├── state.rs        全局共享状态 AppState
    ├── updater.rs      后台定期更新任务
    └── handler.rs      各 HTTP 处理函数（≈ Gin 的 HandlerFunc）
```

真实服务建议 lib + bin 布局：核心逻辑放 lib，`main.rs` 只做组装。好处：逻辑可被别的 crate 复用（[《中间件与优雅退出》](middleware-shutdown.md) 直接依赖本课的 lib）、  
可被集成测试依赖、职责清晰。

## AppState 共享状态

```rust
#[derive(Clone)]
pub struct AppState {
    pub data: Arc<RwLock<Snapshot>>,  // 内存数据：读多写少 → RwLock
    pub reads: Arc<AtomicU64>,        // 读取计数：原子量
}
```

为什么必须 Clone、clone 为什么便宜：axum 为 **每个请求** clone 一份 State 交给 handler；  
字段全是 Arc，clone 只是计数 +1， **不复制底层数据** ——所有请求拿到的都是指向同一份数据的句柄，  
正是 [《共享状态：Arc / RwLock》](../async/shared-state.md) 「Arc 共享所有权」在 Web 场景的落地。

对照 Gin：你会把 `*sql.DB`、缓存、配置塞进一个结构体，靠闭包/方法值捕获注入 handler。axum 用 `.with_state(state)` + 参数 `State<AppState>` 做同一件事，  
但注入是 **类型驱动、编译期检查** 的——类型不匹配直接编译不过。

## 后台更新任务

```rust
tokio::spawn(run_updater(state.clone(), Duration::from_secs(3)));
```

`run_updater` 内部 = [《Tokio 运行时》](../async/tokio.md) interval + [《共享状态：Arc / RwLock》](../async/shared-state.md) 「锁外算好、  
锁内整体替换」：

```rust
let mut ticker = tokio::time::interval(period);
loop {
    ticker.tick().await;
    let fresh = /* 锁外重新计算 */;
    *state.data.write().unwrap() = fresh;   // 写锁只占一瞬
}
```

后台任务和 HTTP 服务是两个独立并发部分，通过共享的 AppState 通信——「定期写 + 高并发读」在服务里的样子。  
对照 Gin：`go func(){ for range ticker.C {...} }()`，完全一致。

## Handler 是接口契约

```rust
// GET /data
pub async fn get_data(State(state): State<AppState>) -> Json<Snapshot> {
    Json(state.snapshot())
}
```

对比一下 step1 里我们手工做的事，axum 全自动化了：

| 手写版（step1~3） | axum 版 |
| --- | --- |
| `split_whitespace().nth(1)` 抠路径 | Router 匹配 + `Path<T>` 提取器 |
| `match path` 手工路由 | `.route("/data", get(h))` |
| 手拼响应字符串、算 Content-Length | 返回 `Json<T>`，自动序列化+设头 |
| 没有的：类型化参数、错误处理 | 提取器体系 + `IntoResponse` |

**参数 = 提取器（Extractor）**。handler 每个参数都必须实现 `FromRequest` 或 `FromRequestParts`：

| 提取器 | 取什么 | 底层 trait |
| --- | --- | --- |
| `State<T>` | 全局共享状态 | `FromRequestParts` |
| `Path<T>` | URL 路径参数 | `FromRequestParts` |
| `Query<T>` | 查询串 `?a=1` | `FromRequestParts` |
| `Json<T>` | 请求体（JSON） | `FromRequest`（**消费 body**） |

区别很重要：`FromRequestParts` 只读请求元信息，可以有多个；`FromRequest` 会 **消费掉请求体**（body 是一次性流），  
一个 handler 最多一个、且必须放参数列表最后（[《读写接口与错误处理》](rest.md) 细讲）。

**返回值 = IntoResponse**。axum 为一大堆类型实现了它：`&str`、`String`、`Json<T>`、  
`(StatusCode, T)`、`Result<T, E>`……所以 `Json(state.snapshot())` 自动序列化 + 设 Content-Type（`Snapshot` 靠 `#[derive(Serialize)]`，  
≈ Go 的 json tag）。

对照 Gin 的哲学差异：Gin 把 `*gin.Context` 传进来你 **手动** 取参数、手动写响应；axum 让你在 **函数签名** 里声明「要什么、  
产出什么」，框架搬运，取错 **编译期** 就报错（Gin 是运行时才发现 `c.Query` 拿到空串）。

## 路由与启动

```rust
let app = Router::new()
    .route("/", get(handler::health))
    .route("/data", get(handler::get_data))
    .route("/stats", get(handler::get_stats))
    .with_state(state);

let listener = tokio::net::TcpListener::bind("0.0.0.0:7080").await.unwrap();
axum::serve(listener, app).await.unwrap();
```

- `.route(path, get(h))`：该路径的 GET 交给 h；同路径多方法 `get(h1).post(h2)`；
- `.with_state(state)`：把状态绑定到整棵路由。类型上的效果：`Router<AppState>`（还缺状态）→ `Router<()>`（可以 serve 了）——状态给没给全，  
  类型可见；
- 注意最后两行和 step3 的 main 多像：**bind 一个 TcpListener、把「处理逻辑」接上去**。  
  差别只是 step3 接的是我们 40 行的手工 handle，这里接的是 hyper+axum 这台工业机器。

**一个请求的完整生命周期**（把前面全串起来）：

```
TCP 连接进来（step1~3 我们手工 accept 的那层）
  → hyper 解析出 HTTP 请求（step1 我们手工 split 的那层，但完整、健壮）
  → Router 匹配路径/方法，选中 handler（step1 的 match path，但支持参数/方法/嵌套）
  → 依次运行 handler 各参数的提取器；任一失败 → 直接回 400/404/422，不进 handler
  → 调用你的 async handler
  → 返回值走 IntoResponse 编码成响应（step1 我们手工 format! 的那层）
  → hyper 写回连接（并处理 keep-alive，而不是像我们那样一律 close）
```

----

# 跑起来看看

> 跑正式版，观察后台刷新和原子计数在工作。

```bash
cargo run -p http-http-from-scratch
```

另开终端：

```bash
curl http://127.0.0.1:7080/          # ok
curl -v http://127.0.0.1:7080/data  # 用 -v 看响应头：这次是 hyper 拼的，比我们手拼的专业
curl http://127.0.0.1:7080/stats    # {"version":N,"item_count":..,"total_reads":..}
```

观察：隔几秒 `/data` 的 version 在涨（后台任务在刷新）；反复 `/stats`，total_reads 在累加（原子计数器在工作）。

压测感受「读写不打架」：

```bash
ab -n 20000 -c 200 http://127.0.0.1:7080/data
# 同时另一个终端观察 updater 日志仍按 3 秒稳定打印
```

看结果重点：**Requests per second**（吞吐）和 **99% 分位延迟**。读走读锁（并发不互斥）、  
写只占一瞬，读延迟应当很稳。 [《ArcSwap 无锁读》](arcswap.md) 换 ArcSwap 后可再压一次对比尾延迟。

----

# Rust vs Gin 对照

> 整体对照一张表。

| 关注点 | Gin（Go） | 本项目（axum + Tokio） |
| --- | --- | --- |
| 运行时/并发 | 语言内建 goroutine | Tokio（显式引入） |
| 底层 HTTP | net/http | hyper |
| 每连接模型 | goroutine（step2 的轻量版） | 异步任务（step3） |
| 中间件模型 | HandlerFunc 链 | tower Service/Layer |
| 后台定时任务 | go + ticker | tokio::spawn(run_updater) |
| 共享数据 | struct + RWMutex | `Arc<RwLock<Snapshot>>` |
| 依赖注入 | 闭包/中间件捕获 | `State<T>`（编译期校验） |
| 取参数 | 手动 c.Xxx（运行时） | 提取器（编译期） |
| 写响应 | c.JSON(200, x) | 返回 `Json<T>` |
| 错误处理 | AbortWithError/手动 return | `Result<T, E>`（[《读写接口与错误处理》](rest.md)） |

----

# 动手实验

1. **看协议原文**：分别对 step1 和 axum 版执行 `curl -v http://127.0.0.1:7080/`，逐行对比两者的响应头；
2. **亲手体会单线程之痛**：用 step1 复现「/slow 卡住所有人」，再用 step2/step3 验证已修复；
3. **在 step3 里踩一次铁律**：把 `/slow` 的 `tokio::time::sleep(...).await` 换成 `std::thread::sleep(...)`，  
   并发打 `/slow` 和 `/`——多线程运行时下可能还好，加上 `#[tokio::main(flavor = "current_thread")]` 再试，  
   `/` 被卡死。改回来；
4. **给 step3 加一个路径**：比如 `/time` 返回当前时间戳——体会没有框架时「加个接口」要动哪些地方，  
   再对比 axum 版只需一行 `.route(...)` + 一个函数；
5. **进阶练习**（后续课程已实现，可先自己动手再对照）：加写接口 `POST /item`（[《读写接口与错误处理》](rest.md)）、  
   换 ArcSwap（[《ArcSwap 无锁读》](arcswap.md)）、优雅退出与请求日志（[《中间件与优雅退出》](middleware-shutdown.md)）、  
   接 Redis（[《接入 Redis》](redis.md)）。

----

# 三句话带走

1. **HTTP 就是 TCP 上格式固定的文本一问一答**：手写版四件事——accept、读文本、抠路径、拼响应写回。框架不神秘，只是把脏活干得完整又健壮。
2. **并发模型三级跳**：单线程阻塞（一个慢请求卡死全部）→ 每连接一线程（Go net/http 模型，OS 线程贵、  
   C10K 撑不住）→ 每连接一个异步任务（几百字节状态机，tokio 的答案）。
3. **分层各司其职**：tokio 管运行时和 TCP，hyper 管 HTTP 协议，tower 管中间件抽象，  
   axum 管路由/提取器/响应——handler 的 **参数是提取器、返回值走 IntoResponse**，在函数签名里声明「要什么、  
   产出什么」，取错编译期报错。

----

# 附：本课生词表

> 通用语法见 [《Rust 语法底座》](../start/syntax-primer.md)；Arc/RwLock/原子量见 [《共享状态：Arc / RwLock》](../async/shared-state.md) 生词表。

- **`std::net::TcpListener` / `bind` / `incoming()`（step1/2）** ——标准库的同步阻塞 TCP 监听：  
  `bind` 占住端口，`incoming()` 是「每来一个连接产出一个 TcpStream」的迭代器；≈ Go 的 `net.Listen` + `ln.Accept()` 循环；  
  tokio 版是 `accept().await` 显式循环。
- **`TcpStream` 与 `Read`/`Write` trait** ——一条 TCP 连接 ≈ Go 的 `net.Conn`；  
  `read` 读进缓冲区无数据就阻塞，`write_all` 全部写出；方法来自 `std::io::{Read, Write}` trait，  
  要 `use`；异步版来自 `tokio::io::{AsyncReadExt, AsyncWriteExt}`，  
  同名但要 `.await`。
- **`String::from_utf8_lossy(&buf[..n])`** ——字节切片 → 字符串：按 UTF-8 解码，  
  非法字节替换为 �（lossy=有损但绝不 panic）；返回 `Cow<str>`。
- **`lines()` / `split_whitespace()` / `nth(1)`** ——手工解析三件套：  
  按行迭代、按空白切段、取第 2 段 = 从 `GET /path HTTP/1.1` 抠出 /path。
- **`\r\n` 与 `Content-Length`** ——`\r\n` 是 HTTP 行分隔符，`\r\n\r\n` = 头部结束；  
  `Content-Length` 必须是 body 的 **字节** 数（`String::len()` 正是字节数）。
- **`thread::spawn(move || handle(stream))`（step2）** ——标准库开 OS 线程 ≈ `go handleConn(conn)`；  
  `move` 把 stream 所有权搬进线程；`thread::current().id()` 打印线程 ID。
- **`Router::new().route(...).with_state(...)`** ——builder 模式：  
  每个方法消费旧 Router 返回新的；`get(h)` 把 handler 包成「只响应 GET」的路由单元（传函数名，  
  函数即值）；`.with_state` 把状态绑定到整棵路由。
- **`State<T>` 与 `State(state): State<AppState>`** ——axum 的元组结构体 `struct State<T>(pub T)`，  
  写在参数上 = 「把全局状态给我」；`State(state)` 是参数位置解构；对照 Gin 闭包捕获依赖。
- **`Json<T>`（返回值位置）** ——「把 T 序列化成 JSON 作响应体」，自动带 Content-Type ≈ `c.JSON(200, t)`；  
  也能用在参数位置表示「解析请求体」；`#[derive(Serialize)]` 是前提。
- **`&'static str`（health 的返回类型）** ——字符串字面量的类型：编译进二进制、程序整个生命周期有效，  
  所以是 `'static`；handler 能直接返回它，axum 转成 200 + text/plain。
- **`axum::serve(listener, app).await`** ——把 Router（一个大 Service）接到 TcpListener 上开始服务 ≈ `r.Run(":7080")`；  
  和 step3 的 main 结构同构。
- **Cargo 的 `src/bin/` 目录与 `--bin`** ——一个 crate 除了主程序，还可以在 `src/bin/` 下放任意多个独立小程序（每文件一个 bin，  
  名字 = 文件名）；`--bin xxx` 指定跑哪个；不带跑主程序（main.rs）。
