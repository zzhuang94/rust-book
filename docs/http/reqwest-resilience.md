# reqwest 与上游容错

> 代码：`code/http-reqwest-resilience/src/main.rs`　运行：`cargo run -p http-reqwest-resilience`

HTTP 服务不只会接收请求，也经常要请求配置中心、节点列表或其他内部服务。
“能发 GET”只是第一步；生产代码还要控制超时、识别状态码、解析 JSON、补充错误现场，
并决定上游暂时不可用时自己的服务是否继续运行。

本课示例会在本机临时启动一个 Axum 上游，再用 `reqwest` 请求它，不依赖互联网和外部服务。  
错误链的基础见 [《通用错误处理》](../lang/error-handling.md)，快照原子替换见
[《ArcSwap 无锁读》](arcswap.md)。

----

# Client 全局复用

创建客户端：

```rust
let client = reqwest::Client::builder()
    .connect_timeout(Duration::from_millis(300))
    .timeout(Duration::from_secs(1))
    .build()?;
```

`reqwest::Client` 内部有连接池。它应该在进程启动时创建一次，放进应用状态，
之后所有请求共享引用或 clone。

不要在每次请求里 `Client::new()`。那会失去连接复用，增加 TCP 建连、端口消耗和延迟。
`Client::clone()` 很轻量，它复制的是共享句柄，不是复制整个连接池。

对应 Go：

```go
client := &http.Client{Timeout: time.Second}
// 把 client 长期复用，不要每个请求重新创建 Transport。
```

> 🔩 底层视角：HTTP/1.1 keep-alive 让多个请求复用同一条 TCP 连接；HTTP/2 还能在一条连接上
> 并发多路请求。频繁重建客户端会把这些能力全部浪费掉。

----

# 两种超时不同

示例设置了两种超时：

- `connect_timeout`：建立连接最多等多久；
- `timeout`：从开始请求到响应体读取完毕的总时限。

只设置连接超时不够。服务器可能很快接受连接，却迟迟不返回响应体。
只设置总超时虽然能兜底，但连接阶段和处理阶段无法使用不同预算。

真实系统还应从整个调用链分配预算。例如入口请求只剩 800ms，
下游调用就不该再给自己独立的 3 秒超时。

超时并不代表对端一定停止工作。客户端放弃等待后，请求可能已经到达上游并产生副作用。
因此自动重试 GET 通常较安全，重试 POST/写操作前必须考虑幂等键和重复提交。

----

# 错误分三层

一次 HTTP 调用至少有三类失败，不要混在一起：

```rust
let response = client
    .get(url)
    .send()
    .await
    .with_context(|| format!("请求上游失败：{url}"))?
    .error_for_status()
    .with_context(|| format!("上游返回非 2xx：{url}"))?;

let data = response
    .json::<Snapshot>()
    .await
    .with_context(|| format!("上游 JSON 格式不对：{url}"))?;
```

逐层解释：

1. `send().await` 失败：DNS、连接拒绝、超时、连接中断等传输问题；
2. `error_for_status()` 失败：HTTP 已成功到达，但状态码是 4xx 或 5xx；
3. `json::<T>().await` 失败：状态码成功，但响应体不是约定的数据结构。

一个非常常见的坑是只写 `send().await?`。`reqwest` 默认不会把 500 当成 Rust 的 `Err`；
500 仍是一个成功收到的 HTTP 响应。需要 `error_for_status()` 主动转换。

`anyhow::Context` 不改变底层原因，只在错误链外面补上“当时在做什么”。
日志打印 `{e:#}` 时，可以同时看到业务现场和底层错误。

不要把含密钥的完整 URL、Authorization、Cookie 或响应正文直接放进错误日志。

----

# JSON 直接落类型

上游数据结构定义为：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Snapshot {
    version: u64,
    nodes: Vec<String>,
}
```

然后用：

```rust
response.json::<Snapshot>().await?
```

这会读取响应体、解析 JSON，并校验字段类型。相比先解析成松散的 `Value` 再手动取字段，
类型化结构能把协议约定写进编译器可见的代码里。

但反序列化成功不等于业务合法。还要按协议验证：

- `version` 是否允许回退；
- `nodes` 是否为空或超过合理上限；
- 地址格式是否正确；
- 是否出现重复项。

外部或不可信响应还应限制响应体大小，避免对方返回超大 JSON 耗尽内存。

----

# 本地上游方便测试

示例自己启动一个 Axum 上游：

```rust
let listener = TcpListener::bind("127.0.0.1:0").await?;
let addr = listener.local_addr()?;
let app = Router::new().route("/snapshot", get(mock_upstream));
let server = tokio::spawn(async move { axum::serve(listener, app).await });
```

这里仍使用端口 0，让操作系统挑空闲端口。测试不依赖固定端口，也不依赖公网。

本地假服务比 mock 掉 `reqwest` 方法更接近真实情况，因为它确实经过 TCP、HTTP 状态码、
响应头和 JSON 解码。之后可以很容易增加失败路由：

- 返回 500，验证状态码错误；
- 延迟两秒，验证总超时；
- 返回残缺 JSON，验证反序列化错误；
- 返回空节点，验证业务校验。

结束时示例调用 `server.abort()`，因为这个假上游只服务本次演示。
真实服务应使用上一课的取消、drain 和优雅退出流程。

----

# 硬依赖与软依赖

上游失败后怎么办，不是 `reqwest` 能替你决定的。先区分两类依赖：

| 类型 | 失败时策略 | 例子 |
| --- | --- | --- |
| 硬依赖 | 当前操作失败，必要时拒绝启动 | 无它就无法验证身份的核心服务 |
| 软依赖 | 保留旧值、降级或稍后重试 | 定期刷新的节点列表、展示配置 |

示例把节点快照当成软依赖，用 `ArcSwap` 保存当前值：

```rust
match fetch(&client, &url).await {
    Ok(fresh) => cache.store(Arc::new(fresh)),
    Err(e) => logln!("刷新失败，继续使用旧快照：{e:#}"),
}
```

关键原则是“先完整取得并验证新值，再原子替换”。不要边下载边改共享状态，
否则失败时可能留下半份新数据。

保留旧快照时，至少还要暴露这些可观测信息：

- 最后一次成功刷新时间；
- 当前快照版本和数据年龄；
- 连续失败次数；
- 最近一次错误类别；
- 数据是否已经超过允许的最大陈旧时间。

软依赖不等于永远忽略失败。旧数据超过安全期限后，系统可能需要降级拒绝部分请求，
或者让健康检查失败以触发人工处理。

----

# 重试要有节制

正确的重试通常包含：

1. 只重试临时性错误，例如连接中断、超时、部分 5xx；
2. 指数退避，例如 100ms、200ms、400ms，而不是立即死循环；
3. 加随机抖动，避免所有实例同时重试形成惊群；
4. 限制最大次数或总时间；
5. 每次尝试和整体操作都受超时控制。

不要重试明显永久的错误，例如 400、401、协议字段缺失。重试无法修复配置错误，
只会增加上游压力并拖延暴露问题。

定时刷新还要避免重叠：上一轮没结束时，不要无上限启动下一轮。
最简单的方法是在一个循环中“请求完成后再等下一次”，或者用信号量保证同时只有一轮。

----

# TLS 特性要看清

本课只请求本机 `http://`，所以依赖写成：

```toml
reqwest = { version = "0.12", default-features = false, features = ["json"] }
```

关闭默认特性可以避免本例引入不需要的 TLS 实现。但生产环境请求 `https://` 时，
必须显式选择项目认可的 TLS feature，并检查证书、系统根证书和部署镜像是否匹配。

不要为了“先跑起来”而关闭证书校验。内部服务也可能遭遇错误路由或中间人攻击。

----

# 安全边界

如果 URL 来自用户输入，直接请求会产生 SSRF 风险：攻击者可能让服务访问
`127.0.0.1`、云元数据地址或内网管理接口。

至少要考虑：

- 只允许固定 scheme 和域名白名单；
- 禁止用户控制完整 URL；
- 限制重定向，防止白名单地址跳去内网；
- 限制响应体大小和总耗时；
- 日志中清除凭证与敏感查询参数。

内部固定配置的上游 URL 风险较低，但配置来源仍应受权限控制。

----

# 动手实验

1. 把假上游改成 `(StatusCode::INTERNAL_SERVER_ERROR, "fail")`，观察非 2xx 错误；
2. 在 handler 中 sleep 两秒，观察客户端一秒总超时；
3. 返回字段类型错误的 JSON，观察错误链中的“JSON 格式不对”；
4. 请求成功后停止假上游，再请求一次，确认缓存仍保持上次快照；
5. 给 `Snapshot` 增加 `updated_at`，实现“超过五分钟不允许继续使用”的检查。

----

# 三句话带走

1. `reqwest::Client` 应全进程复用，并同时设置连接超时和请求总超时。
2. `send`、`error_for_status`、`json` 分别处理传输、HTTP 状态和数据格式三层错误。
3. 上游失败策略取决于硬依赖或软依赖；旧值降级必须有数据年龄、告警和最终期限。

----

# 附：本课生词表

- **`reqwest::Client`** —— 可复用、内部维护连接池的异步 HTTP 客户端。
- **连接池（connection pool）** —— 保存可复用连接，减少重复握手和端口消耗。
- **`connect_timeout`** —— 仅限制建立网络连接所用时间。
- **总超时（request timeout）** —— 限制整个请求直到响应体读完的时间。
- **`error_for_status()`** —— 把 4xx、5xx HTTP 响应转换为 Rust 错误。
- **反序列化（deserialize）** —— 把 JSON 字节转换为 Rust 类型。
- **硬依赖（hard dependency）** —— 不可用时当前能力无法安全继续的依赖。
- **软依赖（soft dependency）** —— 暂时失败时可用旧值或降级继续的依赖。
- **退避（backoff）** —— 重试间隔逐步增大，避免持续轰击故障服务。
- **抖动（jitter）** —— 在重试间隔中加入随机量，打散多实例同步重试。
- **SSRF** —— 服务端请求伪造；攻击者诱导服务访问本不应开放的内部地址。
