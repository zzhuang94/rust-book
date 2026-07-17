# 错误设计的分水岭

> 代码：[`code/lang-error-handling/`](../../code/lang-error-handling/)　  
> 运行：在 `code/` 目录下执行 `cargo run -p lang-error-handling`

> 前置是 [《Rust 语法底座》](../start/syntax-primer.md)（`Result`/`?`）、 [《类型系统与 trait》](types-traits.md)（enum/`From`）。  
> [《读写接口与错误处理》](../http/rest.md)、 [《接入 Redis》](../http/redis.md) 讲的是 Web 场景（错误 → HTTP 响应）；  
> 这一篇讲 **通用场景**：写一个库、写一个 CLI、写后台服务的非 HTTP 部分，错误该长什么样。

对照物是你在 Go 里的全套习惯：`fmt.Errorf("%w", err)`、`errors.Is`/`errors.As`、  
自定义 error 类型。Rust 把这些做成了两个分工明确的工具——`thiserror` 和 `anyhow`。  
这一篇讲清「什么时候用哪个」。

----

# 先复盘已会的

> 开始之前，把你已经会的一段话过一遍。

可失败的函数返回 `Result<T, E>`；`?` 的意思是「成功就取值、失败就带着错误提前 return」，  
而且它会顺路调用 `From` 做错误类型转换；`unwrap`/`expect` 是「失败就 panic」，学习示例里用、  
生产代码里慎用。 [《接入 Redis》](../http/redis.md) 的 `AppError` 展示过完整闭环：  
enum 建模 + `impl From` + 出口处统一转换（那里是转成 HTTP 响应）。

本篇解决更上游的问题：**`E` 到底该怎么设计？** 答案取决于你写的是库还是应用。

----

# 库和应用诉求相反

> 一个反直觉但极其有用的分野：库和应用对「错误」的诉求几乎是相反的。搞清这一点，你就知道该用哪个工具。

| | 库（被别人调用） | 应用（`main` 在你手里） |
| --- | --- | --- |
| 调用者拿到错误后要干嘛 | **区分处理**：文件没找到就重试、权限错误就报警 | **报告**：打日志、返回 500、退出——极少分支处理 |
| `E` 应该是 | 精确的自定义 enum（可 `match`） | 一个「能装下任何错误」的万能盒子 |
| 附加信息 | 结构化字段 | 一路叠加的上下文文字 |
| 社区标配 | **thiserror** | **anyhow** |

对照 Go：Go 没有这个显式分野，全靠约定——库导出 `var ErrNotFound = errors.New(...)` 或自定义类型让人 `errors.Is`/`As`，  
应用层用 `fmt.Errorf("读配置: %w", err)` 层层包。Rust 把这两种姿势分别工具化了。  
下面各讲一个。

----

# 库用 thiserror

> 写库，要给调用者一个 **精确、可 match** 的错误类型。手写这种 enum 要写一堆模板（`Display`、  
> `Error` trait、每个来源一个 `From`）；`thiserror` 用宏替你全写了。

手写的痛点：一个像样的库错误，要写四块——enum 定义、`impl Display`、`impl std::error::Error`、  
每个来源错误一个 `impl From`，全是模板。`thiserror` 让你用三个属性搞定：

```toml
thiserror = "2"
```

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("配置文件 {path} 不存在")]              // ← 这行就是 Display 实现！
    NotFound { path: String },

    #[error("解析失败: {0}")]                        // {0} = 元组第 0 个字段
    Parse(String),

    #[error("IO 错误")]
    Io(#[from] std::io::Error),                      // ← #[from] 自动生成 From impl
}
```

三个属性各干一件事：

- `#[derive(Error)]`：补上 `std::error::Error` trait 实现（错误界的「身份证」，进了这个体系才能被通用工具处理）；
- `#[error("...")]`：生成 `Display`——错误的人话描述，支持 `{字段}`/`{0}` 内插；
- `#[from]`：生成 `From<io::Error>`——于是库代码里 `let f = File::open(p)?;` 的那个 `?`，  
  能自动把 io 错误装进 `ConfigError::Io`。

调用者拿到的是可 `match` 的精确类型，能针对性处理：

```rust
match load_config(path) {
    Err(ConfigError::NotFound { path }) => create_default(&path),   // 针对某种错误特殊处理
    Err(e) => return Err(e.into()),
    Ok(c) => c,
}
```

对照 Go：`#[error(...)]` ≈ 实现 `Error() string`；`#[from]` ≈ 你手写的 `Unwrap()` + 包装构造；  
`match` 变体 ≈ `errors.As(err, &target)`——但 Rust 的 `match` 有 **穷尽性检查**，  
将来新加一个错误变体时，所有处理点都被编译器点名，一个不漏。

----

# 应用用 anyhow

> 写应用（`main` 在你手里），你通常不想为「读配置 + 连数据库 + 起服务」定义十几个错误类型再互相 `From`——只想让 `?` 一路通畅、  
> 最后打一条信息完整的日志。`anyhow::Error` 就是那个「万能盒子」。

`anyhow::Error` 能装下 **任何** 实现了 `std::error::Error` 的错误：

```toml
anyhow = "1"
```

```rust
use anyhow::{Context, Result};   // 注意：anyhow::Result<T> = Result<T, anyhow::Error>

fn load_app() -> Result<AppConfig> {
    let raw = std::fs::read_to_string("app.toml")
        .context("读取配置文件 app.toml 失败")?;        // ← 给错误叠一层上下文
    let cfg: AppConfig = toml::from_str(&raw)
        .with_context(|| format!("解析配置失败, 内容长度={}", raw.len()))?;
    Ok(cfg)
}

fn main() -> Result<()> {        // main 也能返回 Result！出错会打印整条链
    let cfg = load_app().context("应用启动失败")?;
    run(cfg)
}
```

出错时，`anyhow` 会打印出 **完整的因果链**（逐层展开）：

```
Error: 应用启动失败

Caused by:
    0: 读取配置文件 app.toml 失败
    1: 系统找不到指定的文件。 (os error 2)
```

和 Go 习惯的逐条映射：

| Go | anyhow |
| --- | --- |
| `fmt.Errorf("读配置: %w", err)` | `.context("读配置")` / `.with_context(\|\| ...)` |
| `errors.New("...")` / 临时错误 | `anyhow!("...")` 宏 |
| `if bad { return errors.New(...) }` | `bail!("...")`（= `return Err(anyhow!(...))`） |
| `if !cond { return err }` | `ensure!(cond, "...")` |
| `errors.Is(err, ErrX)` | `err.is::<ConfigError>()` |
| `errors.As(err, &target)` | `err.downcast_ref::<ConfigError>()` |
| 错误链遍历 `errors.Unwrap` | `err.chain()`（一个迭代器， [《迭代器》](iterators.md) 的链随便用） |

`context` 的纪律和 Go 包 err 一样：在「边界」处叠（打开哪个文件、调哪个服务、处理哪个用户），别每层都机械地包一遍废话。

----

# 两者如何协作

> `thiserror` 和 `anyhow` 不是二选一，而是标准分工、无缝衔接。

标准分工：**库返回 `thiserror` 精确错误 → 应用用 `?` 收进 `anyhow` 盒子并叠上下文**。  
因为 `anyhow` 接受一切 `std::error::Error`，所以库的 `thiserror` 错误能被 `?` 直接吸进 `anyhow::Error`，  
中间不用你手写转换。

你的 axum 服务里也是这个用法的变体：内部逻辑用 `anyhow`/`thiserror`，到边界处（handler）转成 [《读写接口与错误处理》](../http/rest.md) 里的 `AppError` → HTTP 响应。

----

# panic 的楚河汉界

> Rust 里 `panic!` 和 `Result` 各有地盘，别越界。这条纪律和 Go 的 `panic`/`error` 完全同构，直接搬。

哪些情况 **该** panic（`unwrap`/`expect`/`assert`）：

- **程序员断言**：走到这里 `Some` 一定成立（比如刚 `insert` 过马上 `get`）——用 `expect("刚插入过，必然存在")` 把断言理由写进去；
- **初始化阶段的快速失败**：配置端口非法、必须的环境变量缺失——起都起不来，panic 挺好（各课 `main` 里的 `unwrap` 属于此类）；
- **测试代码**：`unwrap` 随便用。

哪些情况 **绝不** panic：一切「外界输入导致的失败」——用户请求、文件内容、网络响应、解析结果。这些是 `Result` 的领地。

两个补充：

- **库里 panic 是重罪**：你一 panic，调用者整个线程陪葬（在 tokio 里表现为任务变成 `JoinError`，  
  见 [《Tokio 运行时》](../async/tokio.md)）——库必须返回 `Result`，把选择权交给调用者；
- `catch_unwind` 能接住 panic，但它是给框架用的边界设施（web 框架防止一个请求炸掉整个进程），  
  **不是** try/catch，别拿它做业务流程控制。

----

# 三个观念校准

> 给 Go 程序员的三个观念转变，第一周会不适应，之后回不去。

1. **「忘了检查 err」在 Rust 不存在**：Go 的 `val, _ := f()`、或干脆不接收，是静默吞错的头号来源；  
   Rust 里 `Result` 不处理连值都拿不到，编译器还有 `unused_must_use` 警告兜底。你想吞，  
   也得写出 `let _ = f();` 这种「自首式」代码。
2. **错误类型是 API 的一部分**：Go 返回 `error` 接口，只有文档里才写「可能是什么」；Rust 的 `Result<T, ConfigError>` 把「会怎么失败」写进了签名，  
   `match` 时编译器帮你对账。
3. **没有 `if err != nil` 三行诗**：用了 `?` + `context` 之后，主线代码就是快乐路径的直叙——第一周会不适应「错误去哪了」，  
   之后就回不去了。

----

# 动手实验

> 第 1 个能让你直观体会 thiserror 砍掉了多少模板。

1. **thiserror 重写 AppError**：给 `code/http-redis` 加 thiserror 依赖，  
   把手写的 `Display`/`From` 换成 `#[error]`/`#[from]` 属性（`IntoResponse` 保留手写）——体会砍掉的模板量；
2. **anyhow 链**：写个小 CLI：读文件 → 解析数字 → 做除法，三层各叠一个 `context`，喂坏输入看 `Caused by:` 链怎么逐层展开；
3. **downcast**：在上面的 `anyhow` 错误里 `downcast_ref::<std::io::Error>()` 判断「是不是文件不存在」，  
   对照 Go 的 `errors.As` 写法；
4. **体验 must_use**：调用一个返回 `Result` 的函数但不接收返回值，看编译器警告；再 `let _ =` 显式吞掉，  
   体会「吞错必须留痕」。

----

# 三句话带走

1. **库用 thiserror（精确 enum，可 match、可穷尽检查），应用用 anyhow（万能盒子 + context 链）** ——分工衔接：  
   库产精确错误，应用收进盒子里叠上下文。
2. **`.context()` 就是 Go 的 `%w` 包装，`chain`/`downcast` 就是 `Unwrap`/`Is`/`As`** ——纪律同款：  
   在边界处叠有信息量的上下文。
3. **panic 只留给「不可能发生」与「起不来就别起」**；外界输入的一切失败走 `Result`——和 Go 的 panic/error 纪律完全同构，  
   但 Rust 让「忘检查」和「静默吞」在语言层面消失。

----

# 附：本章生词表

- **`std::error::Error` trait** ——错误类型的「身份证」：有它才能进 `anyhow`、  
  才能当 `Box<dyn Error>`、才有 source 链。
- **`thiserror::Error`（derive）** ——为 enum 生成 `Error`/`Display`/`From` 模板；  
  `#[error("...")]` 写人话，`#[from]` 接通 `?`。
- **`anyhow::Error` / `anyhow::Result<T>`** ——装得下任何错误的盒子及其 `Result` 别名；应用层专用。
- **`.context("...")` / `.with_context(|| ...)`** ——给错误叠一层上下文（后者惰性构造，  
  热路径省 format 开销），≈ `fmt.Errorf("%w")`。
- **`anyhow!` / `bail!` / `ensure!`** ——临时造错 / 造错并立即返回 / 条件不满足即返回——应用层三连。
- **`err.chain()` / `downcast_ref::<E>()`** ——遍历因果链 / 试着取回具体类型，  
  ≈ `errors.Unwrap` 循环 / `errors.As`。
- **`fn main() -> anyhow::Result<()>`** ——`main` 返回 `Result`：  
  出错自动打印整条链并以非零码退出，CLI 标配。
- **`#[must_use]` / `unused_must_use`** ——`Result` 被忽略时的编译器警告；吞错必须写 `let _ =` 留痕。
- **`catch_unwind`** ——接住 panic 的边界设施（框架防单个请求炸掉进程用），不是业务 try/catch。
