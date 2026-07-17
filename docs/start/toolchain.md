# 先把工具链装好

> Go 一个安装包就把编译器、格式化、依赖管理全给你了。Rust 也一样省心，只是入口叫 `rustup`。  
> 这一节先把「装好、能跑」这条最短路径走通，后面几节再逐个拆解每个工具。

Rust 的官方安装器叫 **rustup**。它不是编译器本身，而是一个「工具链管家」：帮你下载、切换、更新 Rust 的编译器和周边工具。  
你可以把它类比成「专门管 Go 版本的 `gvm` / `goenv`」，只不过它是 **官方出品、人人都用** 的那一个。

在 Linux / macOS 上，一条命令装好（Windows 去 <https://rustup.rs> 下 `rustup-init.exe` 双击即可）：

```bash
# 从官网拉取安装脚本并执行。装完它会把 rustc / cargo 等命令加进你的 PATH。
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

装完 **新开一个终端**（让 PATH 生效），验证三件套是否就位：

```bash
rustc --version    # 编译器本体，类比 Go 的 `go`（底层那部分）
cargo --version    # 构建 + 依赖 + 运行的总指挥，类比 `go build`/`go mod` 合体
rustup --version   # 工具链管家本身
```

只要这三条都打印出版本号，你的环境就齐了。下面这张表先建立整体印象—— **每个工具对应 Go 里的什么**：

| Rust 工具 | 干什么 | Go 里的对应 |
| --- | --- | --- |
| `rustup` | 装/切/更新工具链 | `gvm`/`goenv`（但官方且通用） |
| `rustc` | 把 `.rs` 编译成可执行文件 | `go tool compile`（你平时不直接碰） |
| `cargo` | 建项目、编译、跑、测、管依赖 | `go build` + `go run` + `go test` + `go mod` |
| `rustfmt` | 自动格式化代码 | `gofmt` |
| `clippy` | 静态检查、给改进建议 | `go vet` + linter |

> 🔬 **为什么 Rust 要单独搞个 rustup？** Go 长期只有一个「当前版本」，而 Rust 有 stable / beta / nightly 三条发布线，  
> 还支持给不同 CPU 架构交叉编译。工具链多了，就需要一个管家统一管理——这就是 rustup 存在的理由。下一节展开。

----

# rustup 管版本

> 这一节的所有命令你现在 **不用背**，扫一眼知道「遇到某类需求该找 rustup」就行，回头当查询手册用。

**更新到最新的 stable 版本**（Rust 每 6 周发一个新 stable，更新很平滑，基本不会破坏你的代码）：

```bash
rustup update            # 更新所有已安装的工具链到最新
```

**stable 和 nightly**：日常工作、本教程全程只用 **stable**（稳定版）。`nightly`（每日构建）带一些还没定稿的实验特性，  
绝大多数项目用不到。你可以先知道有这么回事：

```bash
rustup toolchain install nightly   # 需要时才装 nightly
rustup default stable              # 把默认工具链设回 stable（保证日常稳定）
```

**装两个必备组件** ——格式化工具 `rustfmt` 和 lint 工具 `clippy`。它们通常随 rustup 默认装好了，若没有：

```bash
rustup component add rustfmt clippy
```

**交叉编译**：想在你的开发机上编出「能在别的平台跑」的二进制（比如本地是 macOS，要编 Linux 静态二进制部署上线），  
先加一个「目标平台」（target）。这一步在 [《构建与部署》](../engineering/build-deploy.md) 一章会真正用到，  
这里只需知道它归 rustup 管：

```bash
# 添加「Linux + musl（静态链接）」这个目标，对照 Go 的 GOOS=linux 交叉编译。
rustup target add x86_64-unknown-linux-musl
```

----

# cargo new 起项目

> `cargo` 是你从今往后 **打交道最多** 的命令。它一手包办：建项目、编译、运行、测试、管依赖。这一节先把「建 + 跑」摸熟。

新建一个项目：

```bash
cargo new hello        # 建一个叫 hello 的「可执行」项目
cd hello
```

对照 Go：这一步约等于 `mkdir hello && cd hello && go mod init hello`，  
但 cargo 还顺手给你生成了目录骨架和一个能跑的 `main`。生成出来长这样：

```
hello/
├── Cargo.toml       # 项目清单：名字、版本、依赖。≈ go.mod
└── src/
    └── main.rs      # 源码入口，里面已有一个能跑的 fn main
```

两个默认文件的内容：

```toml
# Cargo.toml —— 项目的"身份证 + 购物清单"
[package]
name = "hello"
version = "0.1.0"
edition = "2021"      # Rust "语言版次"，下一节细说；类比你选 Go 1.21 还是 1.22 的语法

[dependencies]        # 依赖清单，现在是空的
```

```rust
// src/main.rs —— cargo 默认帮你写好的 hello world
fn main() {
    println!("Hello, world!");
}
```

> 小提醒：`cargo new hello` 默认建的是 **可执行** 项目（有 `main.rs`）。若你要建的是给别人 import 的 **库**，  
> 加 `--lib`：`cargo new mylib --lib`，它会生成 `src/lib.rs` 而不是 `main.rs`。  
> 库 vs 可执行的区别， [《模块、crate 与可见性》](../lang/modules.md) 一章讲透。

----

# cargo run 编译并运行

> Go 里你习惯 `go run main.go`。Rust 里对应的就是 `cargo run`——一条命令编译并运行，改完代码再敲一次即可。

在刚才的 `hello/` 目录里：

```bash
cargo run
```

第一次会看到 cargo 先 `Compiling hello`、再 `Running`，最后打印 `Hello, world!`。  
之后没改代码再跑，它发现无需重编，直接运行。

四个每天都会用到的 cargo 子命令，和 Go 对照记忆：

| 你想干嘛 | cargo 命令 | Go 里对应 |
| --- | --- | --- |
| 编译并运行 | `cargo run` | `go run .` |
| 只编译、不运行 | `cargo build` | `go build` |
| 只做类型检查（最快，不生成二进制） | `cargo check` | `go vet` 近似（但更彻底） |
| 跑测试 | `cargo test` | `go test ./...` |
| 出正式发布版（开优化，编译慢、跑得快） | `cargo build --release` | `go build`（Go 默认就带优化） |

> ⚠️ **`cargo check` 是你会爱上的命令**。它只做「能不能编过」的检查，不生成最终二进制，比 `cargo build` 快很多。  
> 写 Rust 的日常节奏是：改代码 → `cargo check` 看编译器怎么说 → 改 → 再 check，  
> 直到过了再 `cargo run`。Rust 编译器的报错极其详细（常常直接告诉你怎么改），`cargo check` 就是和它对话的快捷方式。

## 第一个示例 code/start-toolchain

本书所有示例都在 `code/` 这个工作空间里。第一个就是 `code/start-toolchain`，它比默认的 hello world 多演示了变量、  
函数、遍历，每行都有保姆级注释。完整源码见 [`code/start-toolchain/src/main.rs`](../../code/start-toolchain/src/main.rs)，  
核心片段：

```rust
fn main() {
    println!("Hello, Rust!");

    // let 声明变量，默认「不可变」——这点和 Go 相反，想改必须写 let mut。
    let name = "Gopher";
    // {name} 把同名变量直接插进字符串（对照 Go 的 fmt.Printf("%s", name)）。
    println!("你好，{name}！");

    let sum = add(3, 4);        // 调用下面的函数
    println!("3 + 4 = {sum}");
}

// 参数类型写在冒号后；返回类型用 -> 标注。
fn add(a: i32, b: i32) -> i32 {
    a + b   // ★ 最后一行不带分号 = 返回值，等价于 Go 的 return a + b
}
```

在 `code/` 目录下跑它（`-p hello` 里的 `hello` 是这个 crate 的 package 名）：

```bash
cd code
cargo run -p start-toolchain
```

----

# 读懂 Cargo.toml

> 每个 Rust 项目根上都有一个 `Cargo.toml`。看懂它，你就看懂了「这个项目叫啥、依赖了谁、怎么编译」——和读 `go.mod` 是一样的日常动作。

一个稍微完整点的 `Cargo.toml` 长这样，逐段讲清：

```toml
[package]                    # 本项目的身份信息
name = "myservice"           # 包名。`cargo run -p myservice` 里就是用它
version = "0.1.0"            # 语义化版本号
edition = "2021"             # 语言版次（见下方说明）

[dependencies]               # 运行时依赖，≈ go.mod 的 require 块
tokio = "1"                  # 依赖 tokio，版本 1.x（^1 的简写，允许 1 系列内升级）
serde = { version = "1", features = ["derive"] }   # 带「特性开关」的依赖，下一节讲

[dev-dependencies]           # 只在测试/示例里用的依赖，正式二进制不会带上它们

[profile.release]            # 发布版编译参数（可选调优，构建与部署一章细说）
lto = true                   # 链接期优化，换更小更快的二进制
```

关于 **edition（版次）**：这是 Rust 独有、Go 没有的概念。Rust 每几年发一个「edition」（2015 / 2018 / 2021 / 2024…），  
它 **不是编译器版本**，而是「一组语法约定」。新 edition 会引入一些语法改进，但老 edition 的代码永远能继续编译——你可以在同一个项目里让不同 crate 用不同 edition。  
**新项目一律选最新的稳定 edition（本书用 2021）就对了**，无需纠结。

----

# 加一个依赖

> Go 里 `go get github.com/gin-gonic/gin` 就把依赖加进来了。Rust 里对应的是 `cargo add`，  
> 或者直接手写进 `Cargo.toml` 的 `[dependencies]`。

Rust 的开源库都发布在 **crates.io**（相当于 Go 的 pkg.go.dev + proxy 合体）。加一个依赖：

```bash
cargo add serde --features derive    # 加 serde 库，并打开它的 derive 特性
```

这条命令会往 `Cargo.toml` 的 `[dependencies]` 写一行，等价于你手动写：

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
```

**features（特性开关）** 是 Rust 依赖管理里一个 Go 没有、但极其常用的机制：一个库可以把功能拆成一个个开关，  
你只打开用得到的，编出来的二进制就不带没用的部分。本教程里到处都是，比如 `tokio = { features = ["full"] }`（打开 tokio 全部功能）、  
`serde = { features = ["derive"] }`（打开自动派生）。 [《Cargo 生态与依赖管理》](../engineering/cargo-ecosystem.md) 一章会专门讲。

加完依赖后再 `cargo build`，cargo 会自动去 crates.io 下载它和它的依赖，并把确切版本锁进一个 **`Cargo.lock`** 文件——这就是 Rust 版的 `go.sum`（锁定版本、保证团队和 CI 编出完全一致的结果）。

----

# 工作空间管多个 crate

> 真实项目往往不止一个包。Go 有 `go.work` 把多个 module 放一起开发；Rust 有 **workspace（工作空间）**，  
> 而且用得比 Go 频繁得多——本教程的 `code/` 目录就是一个工作空间。

先厘清两个词，它们是 Rust 组织代码的基本单位：

- **crate（箱）**：一次编译的最小单位。一个 crate 要么编成一个可执行文件（有 `main.rs`），  
  要么编成一个库（有 `lib.rs`）。约等于 Go 的「一个 module 里可被独立构建的东西」。
- **workspace（工作空间）**：把多个 crate 放在一起、 **共享同一个 `Cargo.lock` 和同一个 `target/` 构建缓存**、  
  统一管理依赖版本的一顶大帐篷。

本教程的 `code/Cargo.toml` 就是工作空间的根，长这样（节选）：

```toml
[workspace]
resolver = "2"
members = [                  # 成员 crate 列表，每个都是一个子目录
    "labkit",
    "start-toolchain",
    "lang-basics",           # 语言地基：一章一 crate（lang-* / concurrency-* / http-*）
    "async-basics",
    # … 其它章节同样按“文档目录-文件名”命名
]

# 关键好处：依赖版本在这里统一定一次，各成员用 `xxx = { workspace = true }` 引用，
# 不必每个子 crate 各写一遍版本号——避免版本打架。对照 Go 的 go.work + 根 go.mod。
[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
labkit = { path = "labkit" }     # path 依赖：直接依赖同工作空间里的本地 crate
```

于是每个成员 crate 的 `Cargo.toml` 就特别干净，只需写「我用哪几个，版本听工作空间的」：

```toml
# code/async-basics/Cargo.toml
[package]
name = "async-basics"
edition = "2021"

[dependencies]
tokio = { workspace = true }     # 版本 = 工作空间统一定的那个
labkit = { workspace = true }    # 用同工作空间里的 labkit（那个提供 logln! 的通用 crate）
```

在工作空间里，用 `-p`（package）指定跑哪个成员：

```bash
cd code
cargo run -p start-toolchain    # 跑 start-toolchain
cargo run -p async-basics       # 跑 async-basics
cargo build                   # 不带 -p：编译工作空间里所有成员
```

> 🔬 **为什么 Rust 项目爱用工作空间？** 因为共享一个 `target/` 意味着公共依赖只编一次、多个 crate 复用，  
> 省时间省磁盘；共享一个 `Cargo.lock` 意味着所有成员的依赖版本天然一致，不会「A 子项目用 tokio 1.35、  
> B 用 1.38」。这套机制让「一个仓库里放几十个相关 crate」变得很自然——这也是为什么本教程 14 个小程序能整整齐齐待在一个 `code/` 里。

----

# fmt 和 clippy 守规范

> Go 有 `gofmt` 一锤定音的格式化，还有 `go vet`。  
> Rust 的对应物是 `cargo fmt` 和 `cargo clippy`，习惯早点养成，能省掉大量 code review 里的口水。

`cargo fmt` ——自动格式化，风格官方统一，团队不用再吵缩进和换行：

```bash
cargo fmt          # 就地格式化整个项目/工作空间
cargo fmt --check  # 只检查不改动（CI 里常用，格式不对就让流水线失败）
```

> 正因为有 `rustfmt` 一把梭，本教程的源码 **不为了凑行宽而手动折行** ——排版交给 `cargo fmt` 就好。

`cargo clippy` ——比编译器更啰嗦的「资深同事」，会挑出「能编过、但写得不地道」的地方，还常常直接给出改法：

```bash
cargo clippy                    # 静态检查 + 改进建议（默认规则集）
cargo clippy -- -D warnings     # 把警告当错误（CI 里常用，有警告就失败）
cargo clippy -- -W clippy::pedantic   # 再开一套更挑剔的规则（可选）
```

比如你写了 `if x == true`，clippy 会提醒「直接写 `if x` 就好」。刚学 Rust 时，`clippy` 的建议是 **免费的进阶老师**，值得每条都看一眼。

## 配置文件

| 工具 | 配置文件 | 和 Go 的感觉差在哪 |
| --- | --- | --- |
| `cargo fmt` / rustfmt | `rustfmt.toml` 或 `.rustfmt.toml` | `gofmt` **几乎零配置**（刻意不让你吵风格）；rustfmt 默认同样统一，但允许少量旋钮 |
| `cargo clippy` | `clippy.toml` 或 `.clippy.toml`；也可写进 `Cargo.toml` 的 `[lints]` | ≈ `golangci-lint` 的 `.golangci.yml`——调阈值、开/关某条规则 |

**放哪**：放到 **项目根**（本教程就是 `code/`，和根 `Cargo.toml` 同级）。cargo 从当前 crate 往上找，工作空间根放一份即可管全体成员。

**rustfmt.toml 示例**（没有这个文件 = 全用官方默认，绝大多数项目够用）：

```toml
# rustfmt.toml —— 放在 code/ 或你的 crate 根目录
edition = "2021"       # 按哪个 edition 的语法约定来排版
max_width = 100        # 一行最长多少列（默认 100）
# 完整可选项：https://rust-lang.github.io/rustfmt/
```

常用命令搭配：

```bash
cargo fmt                         # 按 rustfmt.toml（若有）格式化
cargo fmt -- --help               # 看 rustfmt 自己支持哪些开关
rustfmt --print-config default    # 把「当前默认配置」整份打印出来，方便对照改
```

**clippy.toml 示例**（调的是「阈值类」规则，比如参数太多算几个才报）：

```toml
# clippy.toml
cognitive-complexity-threshold = 30   # 认知复杂度超过多少才警告（默认 25）
too-many-arguments-threshold = 8      # 函数参数超过多少才警告（默认 7）
# 完整可选项：https://doc.rust-lang.org/clippy/configuration.html
```

**更常见的做法：在 `Cargo.toml` 里声明 lint 级别**（Rust 1.74+，团队项目推荐；对照 golangci 的 `linters.enable`）：

```toml
# 单个 crate 的 Cargo.toml
[lints.rust]
unsafe_code = "forbid"        # 禁止 unsafe

[lints.clippy]
all = { level = "warn", priority = -1 }   # 先把 clippy::all 设为警告
pedantic = { level = "allow", priority = -1 }  # pedantic 太吵，先关掉
unwrap_used = "warn"          # 单独把 unwrap() 提成警告
```

工作空间可以写一次、全体成员继承：

```toml
# code/Cargo.toml（工作空间根）
[workspace.lints.clippy]
unwrap_used = "warn"

# 各成员 Cargo.toml 里写一行即可接上：
[lints]
workspace = true
```

> 新手阶段：**可以不建任何配置文件**，直接 `cargo fmt` + `cargo clippy` 用官方默认。  
> 等团队要对齐行宽、或要把某条 clippy 规则升成 CI 红线时，再加 `rustfmt.toml` / `clippy.toml` / `[lints]`。

----

# 上手常见报错

> Rust 编译器的报错是出了名的详细、友好。这里把新手最容易撞上的几个先「剧透」一遍，撞上时不慌。

**报错 1：命令找不到 `command not found: cargo`**
装完 rustup 没重开终端，PATH 还没生效。 **新开一个终端**，或手动 `source $HOME/.cargo/env` 再试。

**报错 2：`cargo run` 说 `could not find Cargo.toml`**
你不在项目目录里。cargo 命令要在 **含 `Cargo.toml` 的目录**（或其子目录）下执行。本教程记得先 `cd code`。

**报错 3：改了返回值，冒出 `` expected `i32`, found `()` ``**
八成是你在函数最后一行的返回表达式后 **多写了个分号**：

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b;   // ❌ 加了分号，这行就从"返回值"变成了"一条丢弃结果的语句"，函数于是返回 ()（空）
}
```

`()` 读作 unit，是 Rust 的「空类型」，约等于「什么都没有」。去掉那个分号即可。  
这是 Go 程序员最常踩的第一个坑——因为 Go 里 `return` 是必写的关键字，而 Rust 靠「最后一个不带分号的表达式」隐式返回。

----

# 附：本章生词表

按出现顺序，解释本章第一次露面的名词：

- **rustup**：官方工具链管家，负责装/切/更新 Rust 编译器与组件。类比 `goenv`，但人人都用。
- **rustc**：Rust 编译器本体，平时被 cargo 调用，你很少直接碰。
- **cargo**：Rust 的项目/构建/依赖/测试总指挥，日常打交道最多的命令，≈ `go` 命令的大部分职责。
- **crate（箱）**：一次编译的最小单位；要么可执行（`main.rs`），要么是库（`lib.rs`）。
- **package（包）**：`Cargo.toml` 描述的那个东西，含一个或多个 crate；`cargo run -p <名字>` 里的名字就是它。
- **crates.io**：Rust 官方的开源库中央仓库，≈ Go 的 pkg.go.dev + 模块代理。
- **Cargo.toml**：项目清单文件，写明名字/版本/依赖/编译参数，≈ `go.mod`。
- **Cargo.lock**：锁定所有依赖的确切版本，保证可复现构建，≈ `go.sum`。
- **edition（版次）**：一组语法约定（2015/2018/2021/2024…），不是编译器版本；老代码永远能编。Go 无对应物。
- **features（特性开关）**：库把功能拆成的开关，按需打开，未用的不进二进制。Go 无对应物。
- **workspace（工作空间）**：把多个 crate 放一起、共享 `Cargo.lock` 与 `target/`、  
  统一管依赖的容器，≈ `go.work`。
- **profile**：一组编译参数（`dev` 调试版 / `release` 发布版），控制优化等级等。
- **rustfmt / `cargo fmt`**：官方代码格式化工具，≈ `gofmt`。可选配置文件 `rustfmt.toml` / `.rustfmt.toml`。
- **clippy / `cargo clippy`**：官方 lint，挑「不地道」写法并给改法，≈ `go vet` + linter。  
  阈值类配置用 `clippy.toml` / `.clippy.toml`；规则级别更常写在 `Cargo.toml` 的 `[lints.clippy]`。
- **`[lints]` / `[workspace.lints]`**：在清单文件里声明 rustc / clippy 规则级别（allow/warn/deny），团队项目常用。
- **unit 类型 `()`**：Rust 的「空类型」，表示「没有有意义的值」，函数无返回值时返回它。
