# 代码规范与最佳实践

> 纯文档课。目标不是背一份“禁止事项”，而是建立一套能直接落地的判断方法：名字怎么起、文件怎么拆、依赖往哪指、常见问题优先选什么方案。  
> 第一次接手 Rust 项目时，可以把本章最后的检查清单当成代码审查模板。

Rust 的编译器能拦住内存越界、悬垂引用和很多并发错误，但它不会阻止我们写出一个两千行的 `main.rs`，也不会阻止业务层直接调用数据库、HTTP 和环境变量。

所以，**“能编译”只是下限，“半年后别人还能放心修改”才是工程质量。**

本章分四件事讲：

1. 用社区统一的命名方式，让代码看起来像 Rust；
2. 按职责组织模块，让修改一个功能时不必翻遍全仓库；
3. 给常见工程问题一套稳妥的默认方案；
4. 推荐几个“小而美”的开源仓库，学习真实项目怎样落地这些规则。

----

# 先记住总原则

> **名字表达意图，模块隔离变化，类型维护约束，错误保留上下文。**

遇到拿不准的设计，先问四个问题：

- 这个名字能不能让没看过实现的人猜到它做什么？
- 这个模块是否只有一个主要的变化原因？
- 一个非法状态能不能直接被构造出来？如果能，能否用类型挡住？
- 出错以后，日志能不能回答“做什么、对谁做、为什么失败”？

这四个问题比机械地限制“一个函数不能超过 30 行”更有价值。函数 40 行但只完成一件清晰的事，未必有问题；函数只有 10 行却同时读配置、查数据库、改缓存、发通知，仍然耦合严重。

----

# 命名规范

## Rust 的大小写规则

Rust 社区对名字的写法非常统一。先把这张表记住：

| 对象 | 写法 | 示例 |
| --- | --- | --- |
| 变量、函数、方法 | `snake_case` | `user_id`、`load_config()` |
| 模块、文件、crate | `snake_case` | `user_service.rs`、`message_store` |
| 结构体、枚举、trait | `UpperCamelCase` | `UserProfile`、`LoginState`、`Repository` |
| 枚举变体 | `UpperCamelCase` | `LoginState::Authenticated` |
| 常量、静态变量 | `SCREAMING_SNAKE_CASE` | `MAX_RETRY_COUNT` |
| 生命周期 | 短小写字母 | `'a`、`'de` |
| 泛型参数 | 简短大写字母或明确名称 | `T`、`E`、`K`、`V`、`RequestBody` |

```rust
const MAX_RETRY_COUNT: usize = 3;

struct UserProfile {
    user_id: u64,
    display_name: String,
}

enum LoginState {
    Anonymous,
    Authenticated(UserProfile),
}

fn load_user_profile(user_id: u64) -> Option<UserProfile> {
    todo!()
}
```

不要把 Go、Java 的写法原样搬过来：

```rust
// 不推荐
let userId = 42;
struct user_profile;
const maxRetryCount: usize = 3;

// 推荐
let user_id = 42;
struct UserProfile;
const MAX_RETRY_COUNT: usize = 3;
```

这些格式通常交给 `rustfmt` 和编译器 lint 统一，不需要在代码审查里反复争论空格和换行。

## 名字要表达业务含义

初学者最容易写出 `data`、`info`、`obj`、`manager`、`handle()` 这类“好像什么都能装”的名字。它们的问题不是短，而是没有边界。

```rust
// 看名字不知道输入输出是什么
fn handle(data: String) -> String {
    todo!()
}

// 名字已经说明动作、对象和结果
fn normalize_phone_number(raw_phone: &str) -> Result<String, PhoneError> {
    todo!()
}
```

常用命名套路：

- 执行动作的函数用动词开头：`load_user`、`save_order`、`parse_token`；
- 返回布尔值用 `is_`、`has_`、`can_`、`should_`：`is_expired()`、`has_permission()`；
- 集合用复数：`users`、`pending_tasks`；
- 数量写清单位：`timeout_ms`、`cache_ttl_secs`，不要只写 `timeout`；
- ID 写清对象：`user_id`、`request_id`，不要在同一函数里出现三个含义不同的 `id`；
- 缩写按普通单词处理：`HttpClient`、`JsonBody`、`TcpListener`，不要写成 `HTTPClient`。

> `manager`、`helper`、`util` 不是绝对禁止。它们是一个提醒：如果这个模块什么都管，通常还可以继续按业务职责拆分。

## `as_`、`to_`、`into_` 不要混用

Rust API 对转换方法有一套约定：

| 前缀 | 通常表示 | 例子 |
| --- | --- | --- |
| `as_` | 借用视图，便宜，不取得所有权 | `String::as_str()` |
| `to_` | 创建新值，通常会分配或复制 | `str::to_string()` |
| `into_` | 消耗当前值，转成另一个拥有所有权的值 | `into_bytes()` |

```rust
impl UserName {
    fn as_str(&self) -> &str {
        &self.0
    }

    fn into_string(self) -> String {
        self.0
    }
}
```

调用者看到方法名，就能提前判断“这个操作会不会复制”“原值之后还能不能用”。这正是好命名的价值。

## getter 通常不加 `get_`

Rust 更常见的写法是直接用字段含义命名：

```rust
impl User {
    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }
}
```

`get_user()` 不是语法错误。如果它表示“去数据库或远程服务读取一次”，`get_`/`fetch_`/`load_` 反而能提醒调用者这是有成本、可能失败的操作。约定的重点是让名字体现行为，而不是死守字面规则。

## 生命周期和泛型别故意写长

```rust
// 一个简单借用关系，'a 足够清楚
fn first<'a>(left: &'a str, right: &str) -> &'a str {
    left
}

// 泛型角色很多时，名字应更明确
struct Service<Repository, Notifier> {
    repository: Repository,
    notifier: Notifier,
}
```

`T` 适合“任意一个类型”；当同一段代码里出现多个不同角色时，使用 `Request`、`Response`、`Repository` 反而更容易读。

----

# 注释、文档和格式

## 注释解释“为什么”，代码说明“做什么”

```rust
// 不推荐：只是把下一行翻译成中文
// 重试三次
for _ in 0..3 {
    // ...
}

// 推荐：说明这个数字背后的工程原因
// 上游偶尔会在主从切换期间拒绝连接；三次退避可覆盖常见的短暂抖动，
// 同时把最坏等待时间控制在调用方的 2 秒超时内。
for _ in 0..MAX_RETRY_COUNT {
    // ...
}
```

以下内容值得写注释：

- 为什么选择看起来“不直观”的方案；
- 安全前提、并发不变量、协议细节；
- 兼容旧数据或外部系统的特殊处理；
- 临时方案的退出条件和对应问题编号。

以下内容优先通过重命名或拆函数解决，而不是写注释：

- 变量是什么；
- 这一行调用了哪个函数；
- 一段代码从上到下做了哪些显而易见的步骤。

## `//`、`///`、`//!` 的区别

```rust
// 普通实现注释，只给阅读源码的人看。

/// 公共 API 的文档注释。
///
/// `cargo doc` 会把它生成到 API 文档里。
pub fn parse_user_id(raw: &str) -> Result<u64, ParseIntError> {
    raw.parse()
}

//! 模块或 crate 级文档，通常写在文件最上方。
```

公共 API 至少应该说明：

- 它做什么；
- 参数和返回值的关键语义；
- 什么情况下返回错误；
- 是否可能 panic；
- 有无并发、安全或性能方面的前提。

## 把机械规则交给工具

Rust 项目通常统一使用：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

- `rustfmt` 负责排版；
- `clippy` 负责发现常见的低效、冗余和容易出错的写法；
- `-D warnings` 表示把警告当错误，适合放进 CI；
- 团队需要特殊规则时，再在 `rustfmt.toml`、`clippy.toml` 或 crate 根部统一配置。

不要在每个文件里随手堆 `#[allow(...)]`。确实要关闭一条 lint 时，把范围缩到最小，并写清原因：

```rust
#[allow(clippy::too_many_arguments)]
// 这些参数与外部协议字段一一对应，改成配置对象反而会隐藏协议结构。
fn decode_legacy_packet(/* ... */) {
    // ...
}
```

----

# 代码组织：先从小项目开始

## 小项目不要过度分层

一个只有几个功能的命令行程序，可以从下面的结构开始：

```text
src/
├── main.rs       # 解析参数、组装依赖、决定退出码
├── config.rs     # 配置结构与读取
├── command.rs    # 核心操作
└── error.rs      # 对外错误类型
```

此时没必要一上来就建 `controller/service/repository/domain/adapter` 五层目录。**抽象应该解决已经出现的变化，而不是提前想象所有变化。**

一个实用判断：

- 文件还不到 200 行、职责清楚、修改原因一致，可以先不拆；
- 找一个函数需要频繁滚动，或同一文件包含配置、协议、业务、存储等多种职责，就该拆；
- 拆完后如果两个模块一直成对修改，可能拆得太细；
- 如果一个模块被完全不相关的功能同时依赖，可能职责太宽。

## `main.rs` 要薄

`main` 最适合做四件事：

1. 读取配置；
2. 初始化日志、数据库、HTTP 客户端等资源；
3. 把依赖组装成应用；
4. 启动服务并决定进程如何退出。

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    init_tracing(&config.log_level)?;

    let repository = PgUserRepository::connect(&config.database_url).await?;
    let service = UserService::new(repository);
    let app = build_router(service);

    serve(app, config.listen_addr).await
}
```

这里能一眼看到程序由什么组成，但看不到 SQL、路由细节和业务规则。具体逻辑放进各自模块，`main` 只负责接线。

## 中型服务按业务能力组织

项目变大后，可以使用下面这类结构。名字不必一模一样，依赖方向才是重点：

```text
src/
├── main.rs                 # 进程入口：初始化与接线
├── lib.rs                  # 对测试、其他 bin 暴露稳定入口
├── config.rs               # 配置
├── app.rs                  # AppState、路由/服务组装
├── user/
│   ├── mod.rs              # user 模块对外出口
│   ├── model.rs            # User、UserId 等领域类型
│   ├── service.rs          # 注册、查询等业务规则
│   ├── repository.rs       # 业务需要的存储能力（trait）
│   └── error.rs            # 用户域错误
├── auth/
│   ├── mod.rs
│   ├── token.rs
│   └── service.rs
├── api/
│   ├── mod.rs
│   ├── dto.rs              # HTTP 请求/响应结构
│   └── routes.rs           # 路由与 handler
└── infrastructure/
    ├── postgres_user.rs    # repository 的数据库实现
    └── http_notifier.rs    # 外部 HTTP 实现
```

依赖方向可以概括为：

```text
HTTP / CLI / 定时任务
        ↓
    业务服务
        ↓
业务定义的能力（trait）
        ↑
数据库 / Redis / 外部 HTTP 的具体实现
```

上层知道业务，下层知道技术细节；业务服务不应该知道 Axum 的 `Json`、SQLx 的 `PgPool` 或某个 Redis 命令。

## 优先按业务功能聚合，不要只按技术层堆大目录

下面这种结构在项目小时很整齐，项目大后却容易变成“每改一个功能要横跨五个巨大目录”：

```text
controllers/
services/
models/
repositories/
```

更推荐先按 `user/`、`order/`、`auth/` 聚合；每个业务模块内部再按需要拆 `service.rs`、`repository.rs`。这样删除一个功能时，大部分改动都集中在一个目录。

## 控制可见性

Rust 的 `pub` 不只是“能不能调用”，也是架构边界。

```rust
pub struct UserService<R> {
    repository: R,
}

impl<R> UserService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

pub(crate) fn normalize_email(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}
```

建议从最小权限开始：

- 仅当前模块使用：不写 `pub`；
- 同一 crate 内使用：`pub(crate)`；
- 只给父模块使用：`pub(super)`；
- 真正稳定、需要给外部调用：`pub`。

暴露得越少，重构时需要兼容的东西越少。不要为了“以后可能用到”把所有结构体字段都设成 `pub`。

## 用 `lib.rs` 放可复用逻辑

即使最终产物只有一个可执行文件，也建议把核心逻辑放进 library target：

```rust
// src/lib.rs
pub mod config;
pub mod user;

pub fn build_app(/* dependencies */) -> Router {
    todo!()
}
```

`main.rs` 调 `build_app()`，集成测试也调同一个入口。这样测试不需要启动真正的进程，也不会复制一份路由和依赖组装逻辑。这个思路在 [《测试》](testing.md) 和 [《网络集成测试》](network-testing.md) 中会继续使用。

----

# 解耦：把变化隔离在边缘

## 核心规则：纯业务在里面，IO 在外面

所谓“纯业务”，是指输入确定时输出也确定，不直接读环境变量、系统时间、数据库和网络。

```rust
fn calculate_discount(level: MemberLevel, amount: Money) -> Money {
    // 只计算，不查数据库、不发 HTTP、不读全局配置
    todo!()
}
```

纯函数有三个优势：容易理解、容易复用、容易测试。数据库查询、HTTP 请求、文件操作放在边缘模块，由应用层把结果传进来。

## trait 应该由使用方定义

业务服务需要“按 ID 查用户”，就让业务模块声明它需要的最小能力：

```rust
#[async_trait::async_trait]
pub trait UserRepository {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError>;
}

pub struct UserService<R> {
    repository: R,
}

impl<R: UserRepository> UserService<R> {
    pub async fn profile(&self, id: UserId) -> Result<UserProfile, UserError> {
        let user = self
            .repository
            .find_by_id(id)
            .await?
            .ok_or(UserError::NotFound(id))?;

        Ok(UserProfile::from(user))
    }
}
```

PostgreSQL、内存存储或远程 API 都可以实现这个 trait。业务层只依赖“能查用户”这个能力，不依赖 SQLx。

但不要给每个结构体都配一个 trait。只有出现以下需求时再抽：

- 存在两种以上实现；
- 需要隔离数据库、网络、时钟等外部副作用；
- 这是稳定的业务边界；
- 调用方只需要具体类型的一小部分能力。

只有一个实现、也看不到变化点时，直接用具体类型通常更简单。

## 泛型还是 `dyn Trait`

| 选择 | 适合 | 代价 |
| --- | --- | --- |
| `Service<R: Repository>` | 实现在编译期确定；库代码；追求零成本抽象 | 类型签名可能变长；每种实现会单态化 |
| `Arc<dyn Repository>` | 运行时选择实现；应用接线；需要把不同实现放进同一容器 | 一次动态分派；对象安全限制 |

服务端业务代码里，两者性能差异通常不是首要问题。优先选择能让依赖组装和测试更清楚的方式。

## DTO 和领域类型分开

HTTP 请求里的字符串，不应该直接等于业务里的合法值：

```rust
#[derive(Deserialize)]
struct CreateUserRequest {
    email: String,
}

struct EmailAddress(String);

impl TryFrom<String> for EmailAddress {
    type Error = EmailError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let normalized = raw.trim().to_ascii_lowercase();
        if !normalized.contains('@') {
            return Err(EmailError::InvalidFormat);
        }
        Ok(Self(normalized))
    }
}
```

边界层负责把不可信的 `CreateUserRequest` 转成已经校验过的 `EmailAddress`。进入业务层以后，不必在每个函数里重复判断邮箱是否合法。

这叫 **让非法状态难以表示**。同理可以给 `UserId`、`OrderNo`、`Port`、`NonEmptyName` 建立新类型。

## 不要让全局状态偷偷穿透所有模块

不推荐在业务代码里随处读取：

```rust
let timeout = std::env::var("REQUEST_TIMEOUT")?;
```

推荐在入口处读一次并完成校验，再显式传入：

```rust
struct HttpClientConfig {
    timeout: Duration,
    max_retries: usize,
}

let config = AppConfig::from_env()?;
let client = UpstreamClient::new(config.upstream);
```

这样每个对象的依赖都能从构造函数看出来，测试也不必修改进程级环境变量。

----

# 错误处理的工程约定

## 业务错误用 enum

```rust
#[derive(Debug, thiserror::Error)]
enum CreateUserError {
    #[error("email already exists: {0}")]
    EmailAlreadyExists(EmailAddress),

    #[error("user repository failed")]
    Repository(#[from] RepositoryError),
}
```

调用方可以对 `EmailAlreadyExists` 返回 HTTP 409，对 `Repository` 返回 500。错误类型不仅是字符串，也是程序控制流的一部分。

## 应用入口可以使用 `anyhow`

在 `main`、一次性任务、CLI 命令这类“错误最终只需要报告给人”的边界，`anyhow::Result` 很方便：

```rust
async fn run() -> anyhow::Result<()> {
    let config = load_config().context("failed to load application config")?;
    connect_database(&config.database_url)
        .await
        .context("failed to connect to user database")?;
    Ok(())
}
```

简单规则：

- **库和业务边界**：`thiserror` 定义可匹配的具体错误；
- **应用最外层**：`anyhow` 汇总错误并补上下文；
- 不要一进入底层就把所有错误转成字符串，否则上层无法区分“没找到”和“系统故障”。

完整说明见 [《通用错误处理》](../lang/error-handling.md)。

## `unwrap()` 和 `expect()` 的边界

生产路径中，来自用户、网络、文件、数据库的数据都可能异常，不应直接 `unwrap()`。

```rust
// 不推荐：配置缺失时只得到笼统 panic
let port: u16 = std::env::var("PORT").unwrap().parse().unwrap();

// 推荐：保留“读取”和“解析”两个阶段的上下文
let raw_port = std::env::var("PORT").context("PORT is not configured")?;
let port = raw_port.parse::<u16>().context("PORT must be a valid u16")?;
```

以下场景可以接受：

- 测试代码，为了突出断言目标；
- 编译期已经能证明不会失败的固定值，并用 `expect("说明不变量")` 写清理由；
- 进程无法恢复的初始化错误，但通常仍建议让 `main` 返回带上下文的错误。

不要用 panic 表示普通业务失败；panic 更适合“程序内部不变量被破坏”。

----

# 常见问题的默认解决方案

> 下面不是唯一答案，而是没有特殊约束时的稳妥起点。

| 问题 | 优先方案 | 常见误区 |
| --- | --- | --- |
| 多处只读共享数据 | `Arc<T>` | 为了共享就先套 `Mutex` |
| 少量同步可变状态 | `Arc<Mutex<T>>` / `Arc<RwLock<T>>` | 持锁跨 `.await` |
| 异步临界区必须跨 `.await` | `tokio::sync::Mutex` | 所有地方都无脑用异步锁 |
| 最新配置/快照热更新 | `watch`、`ArcSwap` | 每次请求都抢写锁或重读文件 |
| 队列式任务传递 | 有界 `mpsc` | 无界队列吃光内存 |
| 一次性结果 | `oneshot` | 为一个结果建共享可变状态 |
| 广播事件 | `broadcast` | 把工作队列误当广播，导致只有一个消费者收到 |
| 应用取消 | `CancellationToken` | 只 drop `JoinHandle`，误以为任务会停止 |
| 库错误 | `thiserror` | 所有错误都变成 `String` |
| 应用顶层错误 | `anyhow` + `context` | 只写一个 `?`，丢失当前操作语义 |
| 稳定全局对象 | `OnceLock` / 构造时注入 | 随处使用可变 `static` |
| HTTP 客户端 | 长期复用一个 `reqwest::Client` | 每次请求新建客户端和连接池 |
| CPU 密集任务 | `spawn_blocking` 或专用线程池 | 在 Tokio worker 上长时间计算 |
| 并行迭代计算 | `rayon` | 为每个元素手写线程 |
| 超时 | 在调用边界统一 `timeout` | 只给最内层 socket 设置超时 |
| 重试 | 仅重试短暂故障 + 退避 + 总时限 | 对所有错误无限重试 |

相关章节：共享状态见 [《共享状态：Arc / RwLock》](../async/shared-state.md)，通道和任务控制见 [《超时、限流与任务组》](../async/task-control.md)，热更新见 [《通知与热更新》](../async/notify-watch.md)，退出流程见 [《生产任务生命周期》](../async/service-lifecycle.md)。

## 共享不等于可变

```rust
let config = Arc::new(config);
```

如果配置启动后不再变化，`Arc<Config>` 就够了，不需要 `Arc<Mutex<Config>>`。锁只用于“多个执行流确实要修改同一份数据”的情况。

## 持锁范围越小越好

```rust
// 推荐：锁内只完成内存操作，随后立刻释放
let user = {
    let users = state.users.read().expect("users lock poisoned");
    users.get(&user_id).cloned()
};

// 到这里锁已经释放，可以安全地 await
send_audit_log(user).await?;
```

不要拿着 `std::sync::MutexGuard` 跨 `.await`。任务暂停期间锁仍被占用，其他任务可能全部堵住，严重时形成死锁。

## 通道优先有界

```rust
let (tx, rx) = tokio::sync::mpsc::channel::<Job>(256);
```

有界队列满了以后，发送方会等待，这叫 **背压**：下游处理不过来时，上游必须减速。无界队列只是把“系统已经超载”推迟成“内存耗尽”。

容量 `256` 不是魔法数字，要根据任务大小、吞吐和可接受延迟估算，并配监控观察队列长度。

## 重试必须有边界

一次靠谱的重试至少回答四个问题：

1. 哪些错误可重试？连接重置、超时等短暂故障通常可以；参数错误、鉴权失败通常不可以；
2. 最多重试几次？
3. 每次间隔多久？应使用指数退避并加入少量随机抖动；
4. 整个调用的总时限是多少？

还要考虑操作是否 **幂等**。查询通常可安全重试；扣款、创建订单等写操作必须有幂等键或服务端去重，否则一次超时可能变成两次成功。

## 结构化日志不要拼字符串

```rust
tracing::info!(
    user_id = %user_id,
    request_id = %request_id,
    elapsed_ms,
    "user profile loaded"
);
```

字段可以被日志系统检索和聚合；字符串拼接只能靠模糊搜索。密码、令牌、完整 Cookie、身份证号等敏感信息不要进入日志。详细写法见 [《tracing 结构化日志》](tracing.md)。

----

# 一次典型重构：把“大 handler”拆开

假设一个 HTTP handler 同时完成：

1. 解析 JSON；
2. 校验邮箱；
3. 查询数据库；
4. 插入用户；
5. 写 Redis；
6. 发送欢迎通知；
7. 决定 HTTP 状态码。

第一版能工作，但任何一步变化都要改同一个函数。可以按下面的顺序重构：

```text
HTTP handler
  ├─ 把请求 DTO 转成经过校验的业务命令
  ├─ 调用 UserService::create_user(command)
  └─ 把业务结果映射成 HTTP 响应

UserService
  ├─ 执行业务规则
  ├─ 调用 UserRepository 保存
  └─ 调用 Notifier 发送通知

基础设施实现
  ├─ PgUserRepository
  ├─ RedisUserCache
  └─ HttpNotifier
```

重构顺序很重要：

1. 先把纯校验和纯计算抽成函数；
2. 再把数据库、缓存、外部 HTTP 等副作用包进明确对象；
3. 当业务层需要替换或隔离这些对象时，再提炼小 trait；
4. 最后让 handler 只做协议转换。

不要第一天就设计二十个 trait 和五层泛型。**先找到变化点，再建立边界。**

----

# 代码审查清单

提交代码前，可以从上到下过一遍：

## 名字与可读性

- [ ] 类型用 `UpperCamelCase`，函数和变量用 `snake_case`，常量用 `SCREAMING_SNAKE_CASE`；
- [ ] 名字表达业务含义，没有无边界的 `data`、`info`、`manager`；
- [ ] 布尔值能从 `is_`、`has_`、`can_` 等前缀看出含义；
- [ ] 时间、大小、次数等值标明单位；
- [ ] 注释解释“为什么”，没有逐行翻译代码。

## 结构与边界

- [ ] `main.rs` 主要负责初始化和组装；
- [ ] HTTP DTO、数据库模型和领域类型没有混成一个万能结构体；
- [ ] 业务逻辑不直接读取环境变量或依赖具体 Web 框架；
- [ ] `pub` 范围尽可能小，结构体字段没有无理由全部公开；
- [ ] trait 对应真实边界，而不是为了“看起来解耦”给所有类型套接口；
- [ ] 没有循环依赖，核心业务依赖抽象而不是基础设施细节。

## 错误与稳定性

- [ ] 外部输入路径没有随意 `unwrap()`；
- [ ] 错误保留了操作上下文和原始错误链；
- [ ] 普通业务失败使用 `Result`，没有滥用 panic；
- [ ] 重试有错误分类、次数、退避和总超时；
- [ ] 日志包含 request_id 等定位字段，并避开敏感信息。

## 异步与并发

- [ ] 没有持有同步锁跨 `.await`；
- [ ] 队列默认有界，并考虑了背压；
- [ ] 后台任务有取消、等待和超时退出方案；
- [ ] 阻塞 IO 或 CPU 密集计算没有占住 Tokio worker；
- [ ] HTTP 客户端、数据库连接池等昂贵资源被复用。

## 项目卫生

- [ ] 格式由 `rustfmt` 统一；
- [ ] Clippy 警告没有被大范围粗暴关闭；
- [ ] 新增依赖确有必要，features 只开启需要的能力；
- [ ] 公共 API 有 `///` 文档，复杂行为有示例；
- [ ] 行为变化同步更新了文档和测试设计。

----

# 小而美的开源仓库怎么读

> 不要一上来就啃 Rust 编译器、Tokio 或 Servo。大型仓库优秀，但初学阶段很难分辨“项目本质”与“规模带来的复杂度”。

推荐用四遍阅读法：

1. **第一遍只看 README 和 `Cargo.toml`**：它解决什么问题、依赖哪些 crate、有哪些 feature；
2. **第二遍找入口**：CLI 看 `main.rs`，库看 `lib.rs`，服务看 server 启动函数；
3. **第三遍沿一条功能链走到底**：例如“解析一个参数 → 读取文件 → 输出结果”；
4. **第四遍才看测试和错误处理**：作者如何切边界、怎样构造输入、怎样表达失败。

阅读时不要追求每行都懂。先画出“模块负责什么”和“依赖往哪流”，再回头补语法。

## 1. mini-redis：异步服务端首选

仓库：[tokio-rs/mini-redis](https://github.com/tokio-rs/mini-redis)

这是 Tokio 团队专门用于教学的 Redis 客户端和服务端实现。项目明确说明它不完整、不能用于生产，但它刻意保留了真实服务端最值得学习的部分：TCP 连接、协议帧、共享状态、并发限制、发布订阅、优雅退出和时间测试。

建议阅读顺序：

1. `src/bin/server.rs`：进程怎么启动；
2. `src/server.rs`：监听连接、限制并发、管理退出；
3. `src/connection.rs` + `src/frame.rs`：网络字节怎样变成协议对象；
4. `src/db.rs`：多个连接怎样共享状态；
5. `tests/server.rs`：异步服务如何测试。

重点观察：文件名都对应明确职责；协议解析、网络连接和命令执行没有塞进同一个循环。

## 2. confy：小型库如何守住 API 边界

仓库：[rust-cli/confy](https://github.com/rust-cli/confy)

`confy` 解决“把配置结构保存到各操作系统合适的位置”这一件事。代码规模容易进入，适合观察：

- `lib.rs` 如何组织一个小型库的公开 API；
- 如何用 Serde trait bound 接受不同配置类型；
- TOML、YAML、RON 如何通过 feature 隔离；
- 文件路径、序列化和错误怎样形成清晰边界。

先只追踪 `load()` 和 `store()` 两条调用链，不要一开始就研究所有平台差异。

## 3. human-panic：把复杂能力藏在简单入口后面

仓库：[rust-cli/human-panic](https://github.com/rust-cli/human-panic)

这个库为命令行程序生成更友好的 panic 报告。它的入口很小，背后涉及 panic hook、平台信息、报告落盘和宏，适合学习：

- 怎样给调用者提供很小的 API；
- 怎样把平台相关代码隔离；
- 宏只是入口，主要逻辑仍放在普通函数和类型里；
- 面向终端用户的错误信息和面向开发者的诊断信息如何分层。

初学阶段先看 README 用法，再从导出的宏找到普通函数，不必深挖宏展开细节。

## 4. hexyl：完整但克制的 CLI 工具

仓库：[sharkdp/hexyl](https://github.com/sharkdp/hexyl)

`hexyl` 是终端十六进制查看器。它比纯教学项目更接近真实产品，但功能边界仍很明确，适合学习：

- 命令行参数、文件输入与终端输出如何分层；
- 字节分类如何建模；
- 跨平台终端行为怎么组织；
- 快照式输出测试怎样保证显示结果。

建议先选一个最简单的命令参数，从参数定义一路跟到输出函数，不要从颜色和终端兼容细节开始。

## 5. miniserve：从小工具走向完整 HTTP 应用

仓库：[svenstaro/miniserve](https://github.com/svenstaro/miniserve)

`miniserve` 是一个“立即把目录通过 HTTP 分享出去”的工具。它比前四个项目大一些，适合在读完本教程 HTTP 主线后作为进阶材料：

- CLI 配置如何转换成应用配置；
- 路由、静态文件、认证、上传等功能怎样拆开；
- 一个单二进制应用如何兼顾 Windows、Linux 和 macOS；
- 完整项目如何组织发布、文档和 CI。

不要试图一次读完整仓库。只选“启动服务 → 请求一个静态文件 → 返回响应”这条主线，其余功能暂时忽略。

## 阅读难度排序

| 顺序 | 项目 | 先学什么 |
| --- | --- | --- |
| 1 | confy | 小型库 API、错误、feature |
| 2 | human-panic | 简单入口、平台隔离、用户体验 |
| 3 | hexyl | CLI 分层、输入输出、测试 |
| 4 | mini-redis | Tokio、协议、共享状态、退出 |
| 5 | miniserve | 完整 HTTP 应用的工程组织 |

如果目标是服务端开发，可以直接按 `confy → mini-redis → miniserve` 阅读；如果想先练 Rust 基本项目结构，按表格从上到下更平滑。

----

# 延伸规范

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)：公共库 API 的命名、trait、转换、错误和文档约定；
- [Rust Style Guide](https://doc.rust-lang.org/style-guide/)：`rustfmt` 所遵循的官方格式规范；
- [Clippy 文档](https://doc.rust-lang.org/clippy/)：常见 lint 的原因、示例和配置方式；
- [The Rustonomicon](https://doc.rust-lang.org/nomicon/)：只有开始编写 `unsafe` 或底层抽象时再读，不是入门必修。

----

# 三句话带走

1. **遵循社区命名，名字表达意图**：类型 `UpperCamelCase`、函数变量 `snake_case`、常量 `SCREAMING_SNAKE_CASE`；注释解释为什么，不翻译代码。
2. **核心业务放里面，IO 和框架放边缘**：`main` 只接线，DTO 不等于领域类型，trait 对应真实变化边界，`pub` 从最小范围开始。
3. **常见问题先用稳妥默认值**：错误保留类型和上下文、队列有界、锁不跨 await、任务可取消、客户端与连接池复用；先读小项目的一条主线，再挑战大仓库。

----

# 附：本章生词表

- **命名约定（naming convention）** ——社区对类型、函数、常量等名字大小写和语义的共同规则。
- **DTO（Data Transfer Object）** ——边界上传输数据的结构，如 HTTP 请求/响应；它不等于已经校验过的业务对象。
- **领域类型（domain type）** ——表达业务含义和约束的类型，如 `UserId`、`EmailAddress`。
- **依赖方向** ——模块之间“谁知道谁”的关系；核心业务应尽量不知道数据库、Web 框架等具体技术。
- **依赖注入** ——从构造函数或参数把对象需要的依赖传进来，而不是在内部偷偷读取全局状态。
- **新类型（newtype）** ——用单字段结构体包装基础类型，为它增加业务语义和校验，例如 `struct UserId(u64)`。
- **静态分派 / 动态分派** ——泛型在编译期确定具体实现 / `dyn Trait` 在运行时通过虚表选择实现。
- **背压（backpressure）** ——下游处理不过来时，通过有界队列让上游减速，避免无限堆积。
- **幂等（idempotent）** ——同一个操作执行一次或多次，最终效果相同；决定写操作能否安全重试。
- **单态化（monomorphization）** ——编译器为泛型使用到的具体类型生成专用代码，换取静态分派性能。
- **lint** ——对可疑、低效或不符合约定的代码进行静态提示的规则；Clippy 提供大量 Rust lint。
- **panic hook** ——panic 发生时调用的自定义处理函数，可用于调整错误展示和收集诊断信息。
