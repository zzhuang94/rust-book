# 包模型的差异

> 代码：[`code/lang-modules/`](../../code/lang-modules/)　  
> 运行：在 `code/` 目录下执行 `cargo run -p lang-modules`

> Go 的代码组织简单到你几乎不用想：一个目录一个 package，首字母大写就导出。Rust 的模块系统灵活得多，  
> 也因此绕一点。这一章把 `mod`/`pub`/`use`/crate/workspace 一次对着 Go 讲清，  
> 让你不再被「文件怎么变成模块、`pub` 到底给谁看」困住。

先建立三层心智，从小到大：

- **模块（module）**：一个命名空间，用来组织代码；一个文件里能有好几个。
- **crate（箱）**：一次编译的单元，一棵模块树；要么是可执行程序，要么是库。
- **工作空间（workspace）**：把多个 crate 放一起管理的容器。

Go 的 package 大致相当于 Rust 的一个模块 + 一个目录；但 Rust 把「命名空间」和「编译单元」拆成了 module 和 crate 两层——这是最需要重新建立的一个区分。

----

# crate 是编译单元

> crate 是 Rust 编译的最小单位。它有两种：能跑的（bin）和被引用的（lib）。

- **可执行 crate（bin）**：有一个 `fn main`，编译成一个可执行文件。入口通常是 `src/main.rs`。
- **库 crate（lib）**：没有 `main`，编译成一个可被别人 `use` 的库。入口是 `src/lib.rs`。

一个 package（一个 `Cargo.toml`）可以同时含一个 lib 和多个 bin。本书的服务端各课用的就是这个布局：  
`src/lib.rs` 放核心逻辑（可复用、可被测试），`src/main.rs` 或 `src/bin/*.rs` 是薄薄的可执行入口，  
里面 `use 自己的lib::...`。

对照 Go：Go 里「可执行」是 `package main` + `func main`，「库」是其它 package。  
Rust 把这个区分明确成了 lib crate vs bin crate。

----

# 文件即模块

> Rust 里，一个 `.rs` 文件天然就是一个模块，文件名就是模块名。目录 + 一个 `mod.rs`（或同名 `.rs`）组成子模块。

假设一个库 crate 长这样：

```
src/
├── lib.rs          # crate 根
├── handler.rs      # 模块 handler
└── store/
    ├── mod.rs      # 模块 store
    └── redis.rs    # 模块 store::redis
```

它的模块树是：`crate`（根）→ `handler`、`store` → `store::redis`。文件的目录层级，就映射成模块的嵌套层级。

对照 Go：Go 是「一个目录 = 一个 package，目录里所有 `.go` 文件同属一个 package」。  
Rust 更细：**每个文件是一个独立模块**，要嵌套就用目录。所以 Rust 一个 crate 里能有很深的模块树，  
而 Go 靠目录层级 + 短包名。

----

# mod 声明子模块

> 光有文件还不够——你得在父模块里用 `mod` 把子模块「登记」进来，编译器才会去编译它。这是 Go 没有的一步。

```rust
// 在 src/lib.rs（crate 根）里：
mod handler;          // 登记 handler 模块（代码在 handler.rs）
mod store;            // 登记 store 模块（代码在 store/mod.rs）

// 在 src/store/mod.rs 里：
mod redis;            // 登记 store::redis 子模块
```

`mod handler;`（带分号）的意思是「handler 这个模块的代码在另一个文件里，去 `handler.rs` 找」。  
也可以直接把模块内容写在花括号里（适合小模块）：

```rust
mod math {
    pub fn add(a: i32, b: i32) -> i32 { a + b }
}
```

对照 Go：Go 不需要「登记」——把 `.go` 文件放进目录，它就自动属于那个 package。Rust 要你显式 `mod` 一下，  
好处是模块树完全由代码控制、一目了然，坏处是多写一行。

----

# pub 控制可见性

> Rust 里 **一切默认私有** ——模块、函数、结构体、字段，不加 `pub` 就只有本模块（及子模块）能用。想让外面看到，一路 `pub` 出去。

```rust
mod store {
    pub struct Config {        // pub：结构体对外可见
        pub host: String,      // 字段也要单独 pub，否则外面读不到
        port: u16,             // 没 pub：只有 store 模块内部能访问
    }

    pub fn connect() {}        // pub：函数对外可见
    fn helper() {}             // 私有：只有 store 内部能调
}
```

几档可见性，够用的三种：

- `pub`：对所有能看到本模块的人可见（最开放）；
- `pub(crate)`：只在 **本 crate 内** 可见——跨模块用，但不对外部依赖暴露；
- 不写：私有，只有本模块和其子模块能用。

对照 Go：Go 用 **首字母大小写** 控制导出——`Config` 导出、`config` 私有，粒度是「包级」。  
Rust 用 `pub` 关键字，粒度更细（能精确到字段），而且多了 `pub(crate)` 这档——正好对应 Go 的 `internal/` 目录（「仅本项目内可用」）。

----

# use 引入路径

> `use` 就是 Go 的 `import`，但更灵活：能精确引入单个函数/类型，也能批量引，还能改名。路径分隔符统一是 `::`。

```rust
use crate::store::Config;        // 从本 crate 引一个类型
use std::collections::HashMap;   // 从标准库引
use tokio::time::sleep;          // 从外部依赖引一个函数
use std::io::{Read, Write};      // 批量引入多个
use std::fmt::Result as FmtResult;  // as 改名，避免和别的 Result 撞
```

对照 Go：Go 的 `import "fmt"` 引的是整个包，用的时候 `fmt.Println`；Rust 的 `use` 可以直接把 `Println` 这一级引进来，  
用的时候不用带前缀。两种风格：引到模块级（`use std::collections;` 然后 `collections::HashMap`）或引到具体项（`use std::collections::HashMap;` 然后直接 `HashMap`）——后者更常见。

----

# self super crate

> 写路径时，三个限定符帮你定位「从哪开始找」。它们相当于文件系统里的 `.`、`..`、`/`。

```rust
use crate::store::Config;   // crate = 从"本 crate 的根"开始（绝对路径）
use self::helper;           // self = 从"当前模块"开始
use super::sibling;         // super = 从"上一层父模块"开始
```

- `crate::` —— 绝对路径，从 crate 根算起，最稳，改动文件位置也不容易断；
- `self::` —— 当前模块内；
- `super::` —— 父模块（像 `..`），常用于子模块回引父模块的东西。

对照 Go：Go 的 import 路径是「模块路径 + 包路径」的全局唯一字符串，没有相对导入。Rust 的 `crate`/`super`/`self` 给了相对定位能力，  
在深模块树里挺方便。

----

# pub use 重导出

> `pub use` 是「引进来的同时再导出去」，用来给你的库搭一个干净的对外门面——用户不用关心内部模块怎么分层。

```rust
// 在 lib.rs 里：内部结构分了好几层
mod store;
mod handler;

// 但对外，我希望用户直接 `use mylib::Config`，而不是 `use mylib::store::inner::Config`
pub use store::Config;          // 重导出：把深处的 Config 提到 crate 根
pub use handler::handle;
```

这样库的使用者写 `use mylib::Config` 就行，你内部怎么重构模块层级都不影响他们。标准库和大量框架都这么干（比如 `tokio` 把一堆内部模块的东西重导出到顶层）。  
对照 Go：Go 没有直接对应物，通常靠「把公开 API 都放在包的顶层文件」来达到类似效果。

----

# 工作空间多 crate

> 真实项目往往由多个 crate 组成。工作空间（workspace）把它们放一起，共享构建缓存和依赖版本。本书的 `code/` 就是一个工作空间。

`code/Cargo.toml` 是工作空间根，列出所有成员；成员之间可以用 `path` 依赖互相引用：

```toml
# code/Cargo.toml
[workspace]
members = ["labkit", "lang-basics", "async-basics", "..."]

[workspace.dependencies]
labkit = { path = "labkit" }     # 一个成员，被其它成员依赖
tokio = { version = "1", features = ["full"] }
```

于是各章示例（如 `lang-basics`）能 `use labkit::logln`，
`http-http-from-scratch` 也能——  
都是同一个工作空间里的本地 crate 互引。  
[《环境与工具链》](../start/toolchain.md) 一章讲过工作空间的构建好处（共享 `target/`、  
统一版本）。约定：一章一个 crate，目录名 = `docs` 子目录 + `-` + 文件名。

对照 Go：这约等于 Go 的多 module 仓库 + `go.work`。差别是 Rust 的工作空间用得更普遍、  
更「一等公民」——一个仓库里放几十个相关 crate 是常态。

----

# 常见报错

> 模块系统最容易撞的几个报错，先剧透。

- **`unresolved import` / `cannot find ...`**：多半是忘了 `mod xxx;` 登记，  
  或路径写错（`crate::` 起点弄错）。先检查父模块里有没有 `mod` 那一行。
- **`... is private`**：想用的东西没 `pub`，或者它的某一层父模块没 `pub`。可见性要一路开到底。
- **结构体能访问、字段却报私有**：结构体 `pub` 了，但字段忘了单独 `pub`——Rust 的字段可见性是独立的。
- **方法调不出来**：常常是「trait 的方法没 `use` 进作用域」，见 [《Rust 语法底座》](../start/syntax-primer.md) ——这不算模块报错，  
  但表现很像。

----

# 三句话带走

1. **module 管命名空间、crate 管编译单元**：文件即模块，`mod` 登记子模块，目录 + `mod.rs` 组成嵌套——比 Go「一目录一 package」更细一层。
2. **一切默认私有，用 `pub` 逐层开放**：`pub`（全开）/ `pub(crate)`（仅本 crate，  
   ≈ Go 的 `internal/`）/ 不写（私有）；字段可见性独立。
3. **`use` 引路径、`crate`/`super`/`self` 相对定位、`pub use` 搭对外门面**；  
   多 crate 用工作空间统一管理（本书 `code/` 即是）。

----

# 附：本章生词表

- **模块（module）** ——组织代码的命名空间；一个文件天然是一个模块，也可用 `mod name { ... }` 内联。
- **crate（箱）** ——一次编译的单元：bin（有 `main`，可执行）或 lib（有 `lib.rs`，被引用）。
- **`mod xxx;`** ——在父模块里「登记」一个子模块，告诉编译器去对应文件找它的代码。
- **`pub` / `pub(crate)`** ——可见性关键字：对外全开 / 仅本 crate 内可见；不写则私有。
- **`use`** ——引入路径到当前作用域，≈ Go 的 `import`；支持 `{a, b}` 批量、`as` 改名。
- **`crate` / `super` / `self`** ——路径限定符：crate 根（绝对）/ 父模块 / 当前模块。
- **`pub use`（重导出）** ——引入并再导出，用于给库搭干净的对外 API 门面。
- **工作空间（workspace）** ——多 crate 容器，共享 `Cargo.lock` 与 `target/`，成员可 `path` 互引。
- **lib + bin 布局** ——`lib.rs` 放可复用核心、`main.rs`/`bin/*.rs` 是薄可执行入口，  
  bin 里 `use 自家lib::...`。
