# 接入 Redis

> 代码：`code/http-redis/`　运行：`cargo run -p http-redis`（需先启动 Redis）

前几课的「内存数据」是 **单进程私有** 的：进程一重启就没了；多开一个实例，各存各的。接入 Redis 后，数据变成 **跨进程、跨实例共享且可持久** ——服务水平扩容时的常见形态。本课还会完整讲透 **AppError 错误处理模式**（[《读写接口与错误处理》](rest.md) 埋的伏笔）。

----

# 先准备 Redis

```bash
docker run --rm -p 6379:6379 redis
# 或本机已装的 redis-server
```

验证：`redis-cli ping` 应返回 `PONG`。

----

# redis crate 选型

> Rust 生态最主流的客户端就是 [`redis`](https://crates.io/crates/redis) crate ≈ go-redis。

```toml
redis = { version = "1", features = ["tokio-comp", "connection-manager"] }
```

两个 feature 的含义：

- `tokio-comp`：跑在 Tokio 上（异步 IO）。不开它只有同步 API；
- `connection-manager`：启用 `ConnectionManager`——多路复用、自带重连、可 Clone 的连接。

对照 Go：≈ `import "github.com/redis/go-redis/v9"`；`ConnectionManager` 的角色 ≈ `*redis.Client`（并发安全、  
全局建一个、到处共享）。

> ⚠️ **版本提示**：redis crate 已发布 1.0（API 进入稳定期），本课按 1.x 写。看老教程（0.2x 时代）时 API 偶有出入；  
> 报错就 `cargo doc -p redis --open` 查当前签名，改动通常很小。

----

# 连接不用自己写池

> 为什么不用自己写连接池？因为 `ConnectionManager` 是「单连接多路复用」。

```rust
use redis::aio::ConnectionManager;

#[derive(Clone)]
pub struct AppState {
    pub redis: ConnectionManager,
}

impl AppState {
    pub async fn connect(url: &str) -> redis::RedisResult<Self> {
        let client = redis::Client::open(url)?;            // 只解析地址
        let redis = ConnectionManager::new(client).await?; // 这步才连
        Ok(AppState { redis })
    }
}
```

三个关键特性：

1. **多路复用（multiplexing）**：内部是 **一条** TCP 连接，靠「请求-响应按序配对」+内部排队，  
   允许大量并发请求共用—— **不需要连接池** 就能高并发；
2. **可 Clone**：clone 只是共享底层连接的句柄（内部 Arc + channel），所以能直接当 axum 的 State；
3. **自带重连**：断线自动重连，期间命令得到错误或等待。

对照 Go 的差异（值得知道）：

| | go-redis `*redis.Client` | redis crate `ConnectionManager` |
| --- | --- | --- |
| 模型 | **连接池**（默认 10×CPU 条） | **单连接多路复用** |
| 慢/阻塞命令多 | 更抗打 | 会队头阻塞 |
| 海量短平快命令 | 连接开销大些 | 更省 |

需要连接池语义（如 BLPOP）时，Rust 侧用 `deadpool-redis` / `bb8-redis`。  
本课用 ConnectionManager：最简单且够用。

handler 里怎么用：

```rust
let mut con = state.redis.clone();  // clone 一个句柄（共享底层连接）
let n: i64 = con.incr("lab:counter", 1).await?;
```

为什么 clone 再用：命令方法签名是 `&mut self`（要独占句柄写请求），state 里的是共享引用，  
clone 出自己的句柄才能 `mut`；这个 clone 极廉价，是 ConnectionManager 的标准用法。

----

# AsyncCommands 命令

> `use redis::AsyncCommands;` 之后，连接对象就有了类型化异步方法。

| Rust | Redis 命令 | go-redis |
| --- | --- | --- |
| `con.get(key).await?` | GET | `rdb.Get(ctx, key)` |
| `con.set(key, val).await?` | SET | `rdb.Set(ctx, key, val, 0)` |
| `con.set_ex(key, val, ttl).await?` | SETEX | `rdb.Set(..., ttl)` |
| `con.incr(key, 1).await?` | INCR | `rdb.Incr(ctx, key)` |
| `con.del(key).await?` | DEL | `rdb.Del(ctx, key)` |
| `con.expire(key, secs).await?` | EXPIRE | `rdb.Expire(...)` |
| `con.hset(key, f, v).await?` | HSET | `rdb.HSet(...)` |

**原子自增（并发安全的写）**：

```rust
let counter: i64 = con.incr("lab:counter", 1).await?;
```

INCR 是 Redis 的 **原子** 操作：100 个实例、上万并发同时打，也不会丢更新（不会两个请求都读到 5、  
都写回 6）。这是把计数/限流/发号放 Redis 的核心理由。

**读，并优雅处理「键不存在」**：

```rust
let value: Option<String> = con.get(redis_key(&key)).await?;
match value {
    Some(value) => Ok(Json(KvResp { key, value })),
    None => Err(AppError::NotFound(key)),   // → 404
}
```

关键：**用 `Option<String>` 接收**。Redis 里 key 不存在返回 nil → redis crate 映射成 `None`；  
「键不存在」和「Redis 出错」被分成 `None` 和 `Err` 两条路。对照 Go：go-redis 用哨兵错误 `redis.Nil`，  
要写 `if err == redis.Nil` 区分；Rust 用类型直接分开。

**返回类型标注 = 怎么解析回包**：同一个 `.get()`，左边写什么类型就按什么解析：

```rust
let s: String = con.get("k").await?;          // 期望存在且是字符串
let s: Option<String> = con.get("k").await?;  // 不存在 → None（推荐）
let n: i64 = con.get("k").await?;             // 按整数解析
let _: () = con.set("k", "v").await?;         // 写命令：丢弃回包
```

**这是 redis crate 新手最容易卡的点**：忘了标类型 → 编译器抱怨「类型无法推断」（cannot infer type）。

----

# AppError 三块拼图

> 本课重点。Redis 调用可能失败（断网、超时、类型不符）。怎么让失败干净地变成 HTTP 响应？三块拼图。

**拼图一：带数据的枚举，为每种错误建模**

```rust
pub enum AppError {
    Redis(redis::RedisError),   // 底层失败 → 500
    NotFound(String),           // 业务"没找到" → 404
}
```

**拼图二：From——让 `?` 能自动转换**

```rust
impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self { AppError::Redis(err) }
}
```

**拼图三：IntoResponse——让 AppError 能变 HTTP 响应**

```rust
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AppError::Redis(e) => (StatusCode::INTERNAL_SERVER_ERROR,
                                   format!("redis 错误: {e}")),
            AppError::NotFound(k) => (StatusCode::NOT_FOUND,
                                      format!("key 不存在: {k}")),
        };
        (status, Json(ErrBody { error: msg })).into_response()
    }
}
```

然后魔法发生在这个不起眼的 `?` 上：`let counter: i64 = con.incr("lab:counter", 1).await?;`

`?` 的完整含义：incr 成功 → 取出值继续；失败 → 拿到 `RedisError` → 经 `From` 自动转成 `AppError` → 作为本函数返回值提前 return → axum 看到 `Err(AppError)` → 调 IntoResponse → 500/404 响应。  
**一个 `?` 串起「调用→出错→转换→HTTP 响应」整条链，每一环编译期类型安全。**

对照 Gin，这套模式收敛掉的是：

```go
n, err := rdb.Incr(ctx, "lab:counter").Result()
if err != nil {
    c.JSON(500, gin.H{"error": err.Error()}); return
}
```

额外收益：新增一种底层错误忘了处理 → **编译器提醒**（match 必须穷尽）；不存在「忘了 return 继续跑」；错误→状态码的映射集中在一处。

> 这套 AppError [《读写接口与错误处理》](rest.md) 就该用；那里为循序渐进先用了 `(StatusCode, String)`。  
> **真实项目请从一开始就用它。** 想更省模板，可以用 `thiserror` 自动生成 `From`/`Display`——见 [《通用错误处理》](../lang/error-handling.md)。

----

# 跑起来验证

```bash
# 终端 A
docker run --rm -p 6379:6379 redis
# 终端 B
cargo run -p http-redis
```

```bash
curl http://127.0.0.1:7080/counter   # {"counter":1}，再打一次变 2

curl -X POST http://127.0.0.1:7080/kv \
     -H 'content-type: application/json' \
     -d '{"key":"name","value":"tokio","ttl_secs":60}'

curl http://127.0.0.1:7080/kv/name      # {"key":"name","value":"tokio"}
curl http://127.0.0.1:7080/kv/missing   # 404 {"error":"key 不存在: missing"}
```

观察点：杀进程再重启，/counter 从上次的值继续涨——数据在 Redis 里（**持久 + 跨实例**）；  
用 redis-cli 旁观：`get lab:name`、`ttl lab:name`（TTL 在倒计时）。

----

# Redis 版架构

> 回到项目主题：Redis 版「定期更新 + 高并发读」怎么组合。

```
        ┌──────────────┐  每3秒 SET lab:data  ┌─────────┐
        │ 更新任务/实例  │ ───────────────────→ │  Redis  │ ← 共享数据源
        └──────────────┘                      └─────────┘
                                                ▲   ▲   ▲
                               GET lab:data ────┘   │   └──── GET
                                          ┌─────────┴─────────┐
                                       实例A   实例B   实例C …（水平扩容）
```

组合方式：

- **一个实例（或定时任务）** 周期算好数据 `SET lab:data <json>`（可加 TTL）——这是「定期更新」；
- **所有实例** 的读接口 `GET lab:data`——「高并发读」，且多实例数据 **一致**；
- **再加一层本地缓存挡 Redis 压力**：把 [《ArcSwap 无锁读》](arcswap.md) 当本地缓存，每实例一个后台任务定期从 Redis 拉数据刷进本地 ArcSwap，  
  读接口走 **无锁本地内存**、完全不碰 Redis。

于是：**Redis 做跨实例共享数据源，ArcSwap 做本地读加速** —— [《共享状态：Arc / RwLock》](../async/shared-state.md) / [《ArcSwap 无锁读》](arcswap.md) /本课三课串成一条生产级架构（很多「配置中心 + 本地缓存」就是这个做法）。

----

# 再往前一步

> 选读，几个进阶方向。

- **Pipeline**：一次网络往返发多条命令 `redis::pipe().cmd(..).cmd(..).query_async(&mut con).await?`，  
  批量操作大幅降 RTT。≈ go-redis 的 Pipeline；
- **Pub/Sub**：跨实例消息广播/失效通知（「数据更新了，各实例快刷缓存」）；
- **连接池**：需要「借还连接」语义（BLPOP 等阻塞命令）时用 deadpool-redis / bb8-redis；
- **Lua / MULTI-EXEC**：多命令原子操作（限流器、分布式锁）。

----

# 三句话带走

1. **redis crate + ConnectionManager ≈ go-redis 的 \*redis.Client**：  
   全局一个、并发安全、自带重连；模型是 **单连接多路复用**（不同于 go-redis 连接池），无需自己写池。
2. **`use redis::AsyncCommands`** 后命令方法直接用； **返回类型标注决定回包怎么解析**，  
   「键不存在」= `Option::None`（区别于 go-redis 的 redis.Nil）。
3. **AppError + From + IntoResponse** 三块拼图，让 handler 一个 `?` 完成「调用→出错→转换→响应」整条链——axum 项目的标准错误处理姿势。

----

# 附：本课生词表

> 通用语法见 [《Rust 语法底座》](../start/syntax-primer.md)；Result/`?`/trait 与本课关系最大，  
> 建议先回看 [《通用错误处理》](../lang/error-handling.md)。

- **`redis::Client::open(url)`** ——只 **解析并保存** 地址，不建连接（很轻）；返回 `RedisResult<Client>`（= `Result<T, RedisError>` 的别名）。
- **`ConnectionManager::new(client).await`** ——这一步才真正建 TCP 连接；  
  得到可 Clone、多路复用、自动重连的 manager，放 AppState 共享。
- **`use redis::AsyncCommands;`（trait 方法导入！）** ——本课最重要的语法点：  
  `.get/.set/.incr` 定义在 `AsyncCommands` **trait** 上；**trait 不在作用域，  
  方法就点不出来**——忘了这行 use，报错 "no method named `get` found"。
- **泛型返回值 + 类型标注** ——命令方法返回泛型 `RedisResult<RV>`；`let n: i64 = ...` 按整数解析；  
  `let v: Option<String> = ...` 允许不存在； **左边不写类型 → 推断失败 → 编译错误**；  
  写命令丢弃回包 `let _: () = ...`。
- **`state.redis.clone()`（handler 里）** ——命令方法要 `&mut self`，  
  State 里是共享引用；clone 一个自己的句柄（廉价，共享底层连接）再 mut。
- **`enum AppError { Redis(...), NotFound(String) }`** ——Rust 的 enum 变体可以 **携带数据**；  
  错误建模的天然工具：每种错误一个变体、附带上下文；`match self` 时编译器保证 **穷尽所有变体**。
- **`impl From<redis::RedisError> for AppError`** ——`From`：  
  标准库类型转换 trait； **它是 `?` 的搭档**：`?` 遇到错误类型不匹配时自动调 From::from；  
  一行 impl 换来所有调用点免写转换。
- **`impl IntoResponse for AppError`** ——教 axum「这个错误怎么变 HTTP 响应」（状态码映射 + JSON 错误体）。
- **`std::env::var("REDIS_URL")`** ——读环境变量，返回 `Result<String, VarError>`（未设置 = Err）；  
  ≈ `os.Getenv`，但「没设置」是显式 Err 而非空串。
- **`.unwrap_or_else(|_| "默认".to_string())`** ——Result/Option 兜底：  
  Ok/Some 取值；Err/None 执行闭包算默认值；`|_|` = 忽略错误值的闭包参数；姐妹方法 `.unwrap_or(v)`。
- **`format!("lab:{key}")`（key 命名空间）** ——好习惯：所有 key 加统一前缀，避免和同一 Redis 里其他业务撞 key；  
  ≈ go-redis 项目常见的 keyPrefix 约定。
