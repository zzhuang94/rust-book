# 依赖与生态

> 把「加一个库、锁一个版本、开一个 feature」这套日常，和 Go 的 `go get` / `go.mod` / `go.sum` 对照讲清；  
> 再给一张服务端常用 crate 地图，少走选型弯路。

Rust 的开源库都发布在 **crates.io**（相当于 Go 的 pkg.go.dev + 模块代理合体）。  
管理依赖的工具就是 cargo——你已经在 [《环境与工具链》](../start/toolchain.md) 见过它，  
这一章把「依赖」这一面讲透。

----

# cargo add 加依赖

> Go 里 `go get` 加依赖，Rust 里 `cargo add`（或直接手写进 `Cargo.toml`）。

```bash
cargo add serde --features derive    # 加 serde 库，并打开它的 derive 特性
cargo add tokio --features full      # 加 tokio，打开全部功能
cargo add anyhow                     # 只加，用默认特性
```

`cargo add serde --features derive` 会往 `Cargo.toml` 的 `[dependencies]` 写一行，  
等价于你手动写：

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
```

对照 Go：`go get github.com/xxx` 会更新 `go.mod` 的 `require` 块；  
`cargo add` 更新 `Cargo.toml` 的 `[dependencies]`。两者都会顺带解析并锁定传递依赖（Go 记进 `go.sum`，  
Rust 记进 `Cargo.lock`）。删依赖用 `cargo remove xxx`。

----

# 版本与语义化

> 依赖版本用语义化版本（semver）约束。默认的 `"1"` 不是「锁死 1.0」，而是「允许 1 系列内的兼容升级」。

```toml
[dependencies]
tokio = "1"          # ^1：允许 >=1.0.0, <2.0.0（默认，最常用）
serde = "1.0.200"    # ^1.0.200：允许 >=1.0.200, <2.0.0
foo = "~1.2"         # ~1.2：允许 >=1.2.0, <1.3.0（只放行补丁版）
bar = "=1.2.3"       # 精确锁死这一个版本
```

关键认知：`"1"` 前面隐含一个 `^`（脱字号），意思是「主版本不变的前提下，尽量取最新」。这和 Go modules 的默认行为一致——Go 也承诺「同一主版本内向后兼容」（主版本升级要改 import 路径 `/v2`）。  
Rust 的主版本升级则体现在 crates.io 上是不同的版本号，`Cargo.toml` 里改 `"2"` 即可。

想升级依赖：`cargo update`（在 semver 约束内升到最新，改的是 `Cargo.lock`）；  
想升级到新的主版本，改 `Cargo.toml` 里的版本号再 `cargo update`。

----

# features 特性开关

> **features（特性开关）是 Rust 依赖管理里一个 Go 没有、但极其常用的机制。** 一个库把功能拆成一个个开关，  
> 你只打开用得到的，编出来的二进制就不带没用的部分。

本书里到处都是：

```toml
tokio = { version = "1", features = ["full"] }              # 打开 tokio 全部功能
serde = { version = "1", features = ["derive", "rc"] }      # 派生宏 + 支持 Arc 序列化
redis = { version = "1", features = ["tokio-comp", "connection-manager"] }
sqlx  = { version = "0.8", features = ["runtime-tokio", "postgres", "macros"] }
```

几个要点：

- 每个 feature 打开一部分代码/一组可选依赖。比如 serde 的 `derive` 才让你能写 `#[derive(Serialize)]`；  
  不开它 serde 只有手写序列化；
- **关默认特性**：有些库默认开了一堆你不要的（比如 openssl），用 `default-features = false` 关掉，再手动挑：

```toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
```

- 对照 Go：最接近的是 build tags，但远不如 features 结构化——features 是一等公民，  
  有依赖关系、能传递、`cargo` 帮你算全集。

> 🔬 为什么 Rust 需要 features 而 Go 不太需要？因为 Rust 追求「不为没用到的东西付费」：  
> 把可选功能拆成 feature，编译期按需裁剪，二进制里就没有死代码。Go 靠 GC 和运行时反射能力，很多能力是运行时按需的，  
> 编译期裁剪的动力小。

----

# 依赖的几种来源

> 依赖不一定来自 crates.io。三种常见来源。

```toml
[dependencies]
# (1) crates.io（最常见）
serde = "1"

# (2) 本地路径依赖：用同仓库/相邻目录的 crate（工作区内互引全靠它）
labkit = { path = "labkit" }
http-http-from-scratch = { path = "../http-http-from-scratch" }

# (3) git 依赖：直接指向某个仓库（还没发布到 crates.io、或要用某个未发布的修复）
some-lib = { git = "https://github.com/user/some-lib", branch = "main" }
```

另外两类特殊依赖区：

```toml
[dev-dependencies]      # 只在测试/示例/benchmark 里用，正式二进制不带（见测试一课）
tower = { version = "0.5", features = ["util"] }

[build-dependencies]    # 只给构建脚本 build.rs 用
```

对照 Go：`path` 依赖 ≈ `go.mod` 的 `replace` 指向本地；`git` 依赖 ≈ `go get` 一个仓库地址；  
`dev-dependencies` 是 Go 没有的清晰划分（Go 的测试依赖和正式依赖都混在 `go.mod` 里）。

----

# Cargo.lock 锁版本

> `Cargo.lock` 记录 **整棵依赖树里每个 crate 的确切版本**，保证团队和 CI 编出完全一致的结果——就是 Rust 版的 `go.sum`（+ Go 1.17 起 `go.mod` 里的完整依赖图）。

- `Cargo.toml` 写的是 **约束**（`"1"` = 1 系列任意）；`Cargo.lock` 写的是 **结论**（锁定到 `1.38.1`）；
- 该不该提交 `Cargo.lock`？规则和 Go 略不同：**可执行程序（有 main 的 crate / 工作区）提交它**（保证可复现构建）；  
  **纯库 crate 不提交**（让下游自己解析）。本书 `code/` 是工作区，`Cargo.lock` 已提交；
- `cargo build` 时若 `Cargo.lock` 存在就照它来；不存在则现算一份。想主动升级依赖才 `cargo update`。

----

# 常用 crate 地图

> 换了语言，工具箱也得换。这张表把你在 Go 常用的库，对到 Rust 生态里最主流的选择，本书各章用到的都在这。

| 领域 | Go 常用 | Rust 主流 | 本书出处 |
| --- | --- | --- | --- |
| 异步运行时 | 内建 | **tokio** | [《Tokio 运行时》](../async/tokio.md) |
| Web 框架 | gin / echo | **axum**（+ hyper + tower） | [《axum 入门》](../http/axum.md) |
| 序列化 | encoding/json | **serde** + serde_json | 贯穿全书 |
| 数据库 | database/sql / GORM | **sqlx** / SeaORM | [《sqlx 数据库》](../http/sqlx.md) |
| Redis | go-redis | **redis** / deadpool-redis | [《接入 Redis》](../http/redis.md) |
| 日志 | slog / zap | **tracing** + tracing-subscriber | [《tracing 结构化日志》](tracing.md) |
| 错误（库） | fmt.Errorf | **thiserror** | [《通用错误处理》](../lang/error-handling.md) |
| 错误（应用） | fmt.Errorf 链 | **anyhow** | [《通用错误处理》](../lang/error-handling.md) |
| 时间 | time | **chrono** / time / std::time | 贯穿全书 |
| HTTP 客户端 | net/http | **reqwest** | —— |
| 命令行解析 | flag / cobra | **clap** | —— |
| 配置 | viper | **config** / figment | —— |
| 数据并行 | 手写 goroutine | **rayon** | [《Rust 多线程与并发》](../concurrency/threads.md) |
| 随机数 | math/rand | **rand** | —— |
| 唯一 ID | google/uuid | **uuid** | —— |
| 测试断言/mock | testify | 内建 + **mockall** | [《测试》](testing.md) |

选型心法：优先选下载量大、维护活跃、 **纯 Rust 实现** 的库（纯 Rust 依赖能省掉交叉编译时的 C 工具链麻烦，  
见 [《构建与部署》](build-deploy.md)）。看一个 crate 靠不靠谱，上 crates.io 看下载量、  
最近更新时间、docs.rs 文档质量。

----

# 供应链与安全

> 依赖多了，供应链安全就是个正经问题。两个工具。

```bash
cargo install cargo-audit
cargo audit          # 扫描 Cargo.lock 里有没有已知漏洞的依赖版本

cargo install cargo-deny
cargo deny check     # 更全：漏洞 + 许可证合规 + 重复依赖 + 来源白名单
```

- `cargo audit` ≈ Go 的 `govulncheck`：对着漏洞数据库（RustSec）查你的依赖；
- 生产项目常把它俩挂进 CI，依赖有高危漏洞就让流水线红掉；
- 另外 `cargo tree` 能打印整棵依赖树（`cargo tree -d` 找重复依赖），排查「为什么编出来这么大/这个库怎么被引进来的」很有用。

----

# 三句话带走

1. **`cargo add` 加依赖、`Cargo.toml` 写约束（`"1"` = ^1）、`Cargo.lock` 锁结论** ——对应 Go 的 `go get` / `go.mod` / `go.sum`；  
   可执行程序提交 lock，纯库不提交。
2. **features 是 Rust 独有的按需裁剪机制**：`features = ["..."]` 打开、`default-features = false` 关默认；  
   本书的 tokio `full`、serde `derive` 都是它。
3. **依赖三来源**：crates.io / `path`（工作区互引）/ `git`；选型优先纯 Rust、活跃维护的主流库；  
   供应链安全上 `cargo audit`/`cargo deny`。

----

# 附：本章生词表

- **crates.io** ——Rust 官方开源库中央仓库；≈ Go 的 pkg.go.dev + 模块代理。
- **`cargo add` / `cargo remove`** ——增删依赖并更新 `Cargo.toml`；≈ `go get` / 手动删 require。
- **semver 约束（`"1"`/`"~1.2"`/`"=1.2.3"`）** ——版本范围；`"1"` 隐含 `^`（主版本内兼容升级），最常用。
- **features（特性开关）** ——库的可选功能开关，按需打开、编译期裁剪；`default-features = false` 关默认。
- **`Cargo.lock`** ——锁定整棵依赖树的确切版本，保证可复现；可执行 crate 提交、纯库不提交；≈ `go.sum`。
- **`cargo update` / `cargo tree`** ——在约束内升级锁文件 / 打印依赖树（`-d` 查重复）。
- **`path` / `git` 依赖** ——本地目录 / git 仓库来源；`path` ≈ Go 的 `replace` 本地。
- **`[dev-dependencies]` / `[build-dependencies]`** ——只给测试/示例 / 只给 build.rs 用的依赖区。
- **`cargo audit` / `cargo deny`** ——依赖漏洞扫描 / 更全的合规检查（漏洞+许可证+来源）；≈ `govulncheck`。
