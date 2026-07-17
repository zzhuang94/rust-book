# 这一篇怎么用

> 这是全书的「语法后勤站」。它把各章 **反复出现** 的语法、关键字、习惯用法一次讲清；  
> 各章末尾的「生词表」只解释那一章新冒出来的东西，到处都在用的都收在这里。

第一次读，似懂非懂完全没关系——你不需要背下来。真正的用法是：  
往后任何一章卡在某个语法上（`?` 是啥、`move` 干嘛、`::<>` 什么鬼），翻回这一篇对应小节，查完接着走。

它也是专门写给 Go 程序员的：每个 Rust 语法点，都尽量告诉你「Go 里对应什么、差在哪」。  
差异越大的地方（比如「没有 nil」「没有异常」「没有 GC」），讲得越细。

**想按语言地基系统学**：本篇只作速览；专章顺序见侧栏——  
[《基础类型》](../lang/basics.md) → [《流程控制》](../lang/control-flow.md) →  
[《所有权与借用》](../lang/ownership.md) → [《函数与闭包》](../lang/functions-closures.md) →  
[《生命周期》](../lang/lifetimes.md) → …。  
示例约定：一章一个 crate，如 `docs/lang/basics.md` → `code/lang-basics/`，  
运行 `cargo run -p lang-basics`（先 `cd code`）。

----

# crate 与工作空间

> 这是 Rust 组织代码的两个基本单位。搞混它们，`cargo run -p xxx` 里的 `xxx` 到底填什么就会一直迷糊。

| 概念 | 含义 | Go 对照 |
| --- | --- | --- |
| **crate**（箱） | 一次编译的单元/包，有自己的 `Cargo.toml` | module（`go.mod`） |
| **workspace**（工作空间） | 多个 crate 统一管理，根 `Cargo.toml` 列 `members` | 多 module 仓库 + `go.work` |
| `cargo run -p xxx` | 运行工作空间里名叫 `xxx` 的成员（`-p` = package） | 进对应目录 `go run` |
| **feature**（特性开关） | crate 的可选功能开关，按需打开 | 最接近 build tags |

两条要点，现在记个印象， [《环境与工具链》](toolchain.md) 一章有完整展开：

- 根 `Cargo.toml` 的 `[workspace.dependencies]` 统一声明依赖版本；各成员写 `tokio = { workspace = true }` 来引用，  
  于是 **全工作空间版本一致**，不会各用各的版本打架。
- `tokio = { version = "1", features = ["full"] }` 里的 `features` 是按需开启的功能开关——只编你用得到的部分，  
  省编译时间也省二进制体积。

----

# 模块 mod use pub

> Rust 的模块系统和 Go 的 package 长得不太一样。这里先给最小够用的一套，够你读懂各章代码；系统讲解见 [《模块、crate 与可见性》](../lang/modules.md)。

```rust
// 在 lib.rs 里：
pub mod handler;   // 声明一个子模块 handler（它的代码在同目录的 handler.rs）
                   // 前面的 pub = 对外可见（≈ Go 里首字母大写表示导出）

// 在别的文件里：
use crate::state::AppState;  // crate = "当前这个 crate 的根"，从自己项目里引
use tokio::time::sleep;      // 从外部依赖 tokio 里引一个函数
use std::time::Duration;     // std = 标准库
```

逐点说清：

- `use` ≈ Go 的 `import`，但更精细：能精确到 **单个函数/类型**，也能 `use xxx::{a, b}` 一次引好几个。
- 路径分隔符统一是 `::`（不是 Go 的 `.`）。`std::time::Duration` 读作「标准库 → time 模块 → Duration 类型」。
- **不标 `pub` 的东西默认是模块私有的**（≈ Go 里小写开头）。想让外面用得到，就得一路 `pub` 出去。
- **lib + bin 布局**（HTTP 服务几章会用到）：`lib.rs` 是库入口（可被复用），`main.rs` 是可执行入口，  
  bin 里通过 `use http_http_from_scratch::...` 引用自家 lib 的东西。

----

# 两种文档注释

> 各章的每个 `.rs` 文件开头都有 `//!`、每个函数上方都有 `///`。它们不是普通注释，而是能生成 API 文档的「文档注释」——先认识它们，  
> 读代码才不困惑。

| 写法 | 说明的对象 | 位置 |
| --- | --- | --- |
| `//` | 普通注释，纯给读代码的人看 | 任意 |
| `///` | **文档注释**：说明紧跟其后的那个项（函数/结构体/枚举…） | 项的上方 |
| `//!` | **模块级文档注释**：说明「包含它的那个文件/模块」本身 | 文件开头 |

两点补充：

- `///` 和 `//!` 里的内容支持 Markdown。`cargo doc --open` 会把它们生成 HTML 文档——Rust 生态里那些漂亮的 API 文档，  
  全是这么从注释里长出来的。
- 这套机制 ≈ Go 的注释惯例（`// FuncName does...`）+ godoc，但 Rust 是 **专用语法**、支持富文本，地位更正式。

----

# 带感叹号的是宏

> `println!`、`format!`、`vec!`、`tokio::join!`、本书的 `logln!`——凡是名字后面带 `!` 的，  
> **都是宏（macro），不是普通函数**。第一次看到那个 `!` 会以为是「取反」或「重要」，其实它是宏的标志。

为什么 Rust 要有宏这种东西（Go 完全没有）：

- 宏能接受 **可变数量、可变类型** 的参数，比如 `println!("{} {}", a, b)` 想放几个占位符就放几个；
- 宏能 **自创语法**，比如 `select!` 里 `分支 = 条件 => 代码` 这种写法，普通函数根本表达不出来；
- 宏在 **编译期** 展开成普通代码，运行时零额外开销。

对照 Go：Go 没有宏，`fmt.Println` 靠 `...interface{}` 变长参数实现类似效果；  
Rust 则选择在编译期展开。日常使用时， **把宏当成「更灵活的函数」用就行**，不必纠结内部。

## 读懂 labkit 的 logln

本书的 `logln!` 就是一个自定义宏，麻雀虽小五脏俱全（源码见 [`code/labkit/src/lib.rs`](../../code/labkit/src/lib.rs)）。  
你现在 **不需要会写宏**，但要能认出这个结构，因为读开源库时经常撞见：

```rust
#[macro_export]                       // (1) 导出这个宏，让别的 crate 也能用
macro_rules! logln {                  // (2) macro_rules! 定义一个"按模式展开"的宏
    ($($arg:tt)*) => {                // (3) 匹配模式：捕获任意一串 token
        println!("[{}] {}",
            $crate::now(),            // (4) $crate = "定义这个宏的那个 crate"（labkit）
            ::std::format_args!($($arg)*))  // (5) 把捕获到的参数原样转交给格式化
    };
}
```

逐条拆解：

- (1) `#[macro_export]`：宏默认是私有的，加上它才能被 `use labkit::logln` 引用。
- (2) `macro_rules!` 是「声明宏」：左边写「输入长什么样」的匹配模式，右边写「展开成什么」。
- (3) `$($arg:tt)*` 读作「零或多个任意语法单元（token tree）」——正因为这样，`logln!` 才能接受和 `println!` 一模一样的任意参数。
- (4) `$crate` 保证展开后调用的是 **labkit 自己的** `now()`，不管使用方的 crate 里有没有同名函数，都不会认错人。
- (5) `format_args!` 是格式化的底层原语，把参数打包直接转交，避免多一次字符串拷贝。

----

# 属性 #[...]

> 贴在项（函数、结构体…）上方、方括号包起来的 `#[...]`，叫「属性（attribute）」。它是给编译器或宏看的标注，不是运行时代码。

```rust
#[tokio::main]              // 属性宏：把下面的 async main 包进 Tokio 运行时里跑
#[derive(Clone, Serialize)] // derive 宏：自动帮你生成 Clone / Serialize 的实现
#[cfg(unix)]                // 条件编译：只在 Unix 平台编译这段（≈ Go 的 build tag）
```

其中 `#[derive(...)]` 最常见：让编译器替你写那些「千篇一律、手写很烦」的 trait 实现（比如「怎么克隆」「怎么序列化成 JSON」）。  
看到 `#[derive(...)]` 就想：**这行在自动生成代码**。

----

# 没有 nil，用 Option

> 这是 Go 转 Rust 最重要的心智迁移之一：**Rust 里没有 `nil`**。「一个值可能不存在」这件事，必须用类型明明白白写出来。

```rust
enum Option<T> { Some(T), None }   // 标准库自带：要么 Some(有值)，要么 None(没有)
```

- Rust **没有 nil / null**。「可能不存在」只能用 `Option<T>` 表达：有值是 `Some(v)`，没有是 `None`。
- 编译器会 **强迫你处理 `None` 分支** ——所以「忘了判空导致崩溃」这种事，在 Rust 里根本发生不了（编译就过不去）。
- Go 对照：你在 Go 里用 `*T` 判 `nil`、或 `v, ok := m[k]` 判存在。 **关键区别**：  
  Go 忘判 nil 是 **运行时** panic，Rust 忘处理 `None` 是 **编译期** 错误——问题被提前到你还没上线的时候。

----

# 没有异常，用 Result

> 另一个大迁移：**Rust 没有异常（exception）**。可能失败的操作，不靠 `throw`，而是返回一个 `Result`。

```rust
enum Result<T, E> { Ok(T), Err(E) }   // 要么 Ok(成功值)，要么 Err(错误)
```

- 可能失败的函数返回 `Result<T, E>`：成功把结果装在 `Ok` 里，失败把错误装在 `Err` 里。
- Go 对照：`(v, err)` 双返回值。 **关键区别**：Rust 把它做成一个 **类型**，`v` 被包在 `Ok` 里，  
  不「拆开」就拿不到——于是「拿了 v 却忘了看 err」这种 Go 里的经典 bug，在 Rust 里不可能发生。

----

# 拆开 Option 和 Result

> `Option` 和 `Result` 都得「拆开」才能拿到里面的值。下面五种拆法 **贯穿全书**，务必混个脸熟。

```rust
// (1) unwrap：成功就取值；失败 / None 就直接 panic（崩给你看）
let v = result.unwrap();
//   学习示例里大量用它，意思是"出错就当场崩，别藏着"；生产代码里要谨慎。

// (2) expect：和 unwrap 一样，但 panic 时带上一句你写的说明，方便定位
let s = connect(url).await.expect("无法连接 Redis");

// (3) match：全手动，最啰嗦，但每个分支都摊在眼前，最清楚
match maybe {
    Some(v) => println!("有值: {v}"),
    None => println!("没有"),
}

// (4) if let / while let：只关心其中一个分支时的轻量写法
if let Some(v) = maybe { /* 只有是 Some 才进来，v 就是里面的值 */ }
while let Some(m) = rx.recv().await { /* 每收到一条就进一次循环，收到 None 就停 */ }

// (5) ?：在"返回 Result 的函数"里，成功就取值、失败就直接把错误 return 出去
let n: i64 = con.incr("k", 1).await?;
//   这一个 ? 顶 Go 的四行：if err != nil { return err }
```

再补三点，都是高频：

- `?` 遇到「错误类型对不上」时，会自动用 `From` trait 帮你转换（[《接入 Redis》](../http/redis.md) 一章自定义错误的核心机制就是它）。
- 想给「没有值」兜个默认：`.unwrap_or(默认值)` / `.unwrap_or_else(|| 现算一个默认值)`。
- `panic!` ≈ Go 的 `panic`，但 Rust 文化里它只用于「程序员断言：这事绝不该发生」； **可恢复的错误一律走 `Result`**，  
  不用 panic。

----

# trait 就是接口

> `trait` 约等于 Go 的 `interface`，但有一个关键差别：Go 的接口是 **隐式** 满足，Rust 必须 **显式** 声明「我实现了这个 trait」。

```rust
trait Greet {
    fn hello(&self) -> String;
}

impl Greet for MyType {              // 显式声明：MyType 实现了 Greet
    fn hello(&self) -> String { "hi".into() }
}
```

要点：

- Go 里一个类型只要「凑齐了方法」就自动满足接口；Rust 必须白纸黑字写 `impl X for Y`。好处是「谁实现了谁」一目了然，坏处是啰嗦一点。
- 全书高频出现的 trait，先扫一眼混个脸熟：

| trait | 一句话含义 |
| --- | --- |
| `Clone` | 可克隆（`.clone()`） |
| `Send` / `Sync` | 可跨线程移动 / 可跨线程共享引用（并发几章的主角） |
| `Future` | 可以被 `.await` 的「待办卡片」 |
| `Serialize` / `Deserialize` | serde 的序列化 / 反序列化 |
| `IntoResponse` | axum：能变成一个 HTTP 响应 |
| `From` | 类型转换（`?` 自动转错误的搭档） |
| `Default` | 有一个「默认值」 |
| `Drop` | 析构钩子（离开作用域时自动调用） |

- ⚠️ **一个大坑：trait 的方法必须先 `use` 进作用域，才能调用！** 比如 [《接入 Redis》](../http/redis.md) 里，  
  得先 `use redis::AsyncCommands;`，之后连接对象才「多出」`.get()`/`.set()` 这些方法。  
  Go 里没这回事（方法永远跟着类型走），初见很困惑，记住「方法调不出来先看 use 全没全」即可。

----

# 泛型与 impl Trait

> 泛型 ≈ Go 1.18 的泛型，但 Rust 用得更多、更深。这里只给最小认知，专章见 [《泛型与 trait bound》](../lang/generics.md)。

```rust
fn largest<T: PartialOrd>(list: &[T]) -> &T { /* ... */ }
//         ^^^^^^^^^^^^^ 泛型参数 T + 约束「T 必须能比大小」（≈ Go 的类型约束）

async fn say(/* ... */) -> String { /* ... */ }
// 它真实的返回类型其实是 impl Future<Output = String>
//                        ^^^^ 读作"某个实现了 Future 的类型，具体叫啥不重要"
```

两个会反复撞见的小语法：

- **turbofish `::<>`**：当编译器推断不出泛型具体是什么时，用它手动指定，比如 `mpsc::channel::<String>(16)`（明确说「这个 channel 传 String」）。  
  这个奇怪的名字来自 `::<>` 长得像条鱼。
- 多数时候 turbofish 可以省掉——在 **等号左边写类型标注** 同样能帮编译器选定泛型，比如 `let n: i64 = con.incr(...).await?;` 里的 `: i64` 就替编译器定了型。

----

# 闭包和 move

> 闭包 ≈ Go 的匿名函数。真正需要花心思的是那个 `move` 关键字——它和「没有 GC」直接相关，是 `tokio::spawn(async move {...})` 里 `move` 的由来。  
> 专章：[《函数与闭包》](../lang/functions-closures.md)（捕获、`Fn`/`FnMut`/`FnOnce`）。

```rust
let add = |a, b| a + b;              // 闭包：|参数| 函数体，≈ Go 的 func(a, b int) int
let f = move || println!("{name}");  // move：把用到的变量的所有权"搬"进闭包里
```

什么时候 **必须** 写 `move`：

- 当闭包（或 async 块）可能活得 **比当前函数还久** 时——最典型就是把它交给 `tokio::spawn` 丢到后台去跑。
- 这时如果闭包只是「借用」了栈上的变量，函数一返回、栈变量没了，借用就变成悬垂引用——编译器会直接拒绝。把变量 `move` 进去（转移所有权），  
  闭包自己拥有它，就没有悬垂问题了。

对照 Go：`go func(){ use(x) }()` 里你随手就捕获了 `x`，GC 在背后保证 `x` 一直活着。  
Rust 没有 GC，于是用 `move` + 所有权在 **编译期** 解决同一个问题——这就是全书 `async move`、  
`tokio::spawn(async move {...})` 满天飞的原因。

----

# 字符串与迭代器

> 字符串是 Go 程序员进 Rust 的一个小台阶：Rust 把「只读视图」和「拥有者」分成了两个类型 `&str` 和 `String`。  
> 迭代器则是 Rust 处理集合的主力写法。

```rust
let s: &str = "字面量";            // &str：字符串切片（一段只读视图，不拥有数据）
let owned = String::from("拥有");   // String：拥有所有权、可增长
let o2 = "abc".to_string();        // 造 String 的三种等价写法：
let o3: String = "abc".into();     //   String::from / .to_string() / .into()

let s2 = format!("你好, {owned}");  // format!：拼字符串，{} 里可直接写变量名

let v = vec![0i64; 3];             // vec! 的"重复"形态：3 个 0 → [0, 0, 0]
let w = vec![1, 2, 3];             // vec! 的"列举"形态

let squares: Vec<i64> = (0..5).map(|n| (n * n) as i64).collect();
// 区间 0..5 → map 逐个变换 → collect 收集成 Vec，≈ Go 里 for + append 的迭代器版
```

要点，见 [《字符串、数组与切片》](../lang/strings-slices.md)、  
[《集合：Vec 与 HashMap》](../lang/collections.md)、 [《迭代器》](../lang/iterators.md)：

- `&str` vs `String` ≈ 只读视图 vs 拥有者。函数参数常写 `&str`（`String` 和字面量都能传进来，更通用）。
- `{owned}` 这种「把变量名直接写进大括号」的内插（Rust 2021 起支持）等价于 `format!("你好, {}", owned)`。
- `as` 是 **显式** 数值转换，比如 `version as i64`。⚠️ **Rust 数值从不隐式转换**，  
  `i32` 和 `i64` 相加都得先手动转——比 Go 还严格。
- `0..5` 是半开区间（含 0 不含 5）；`0..=5` 是闭区间（含 5）；`0..3u64` 的后缀 `u64` 指定了元素类型。

----

# 智能指针速览

> 「智能指针」听着玄，其实就是「像指针一样用、但自带额外能力（引用计数、加锁…）」的类型。这里一分钟扫过，专章见 [《智能指针全家桶》](../lang/smart-pointers.md)。

| 类型 | 一句话 | 在书里哪出现 |
| --- | --- | --- |
| `Box<T>` | 把值放到堆上，单一所有者 | `Box::pin`（Tokio 一章） |
| `Rc<T>` | 单线程引用计数（不能跨线程） | 仅作对比，实战不用 |
| `Arc<T>` | **原子** 引用计数，可跨线程共享 | 全书主角（共享状态一章细讲） |
| `Mutex<T>` / `RwLock<T>` | 带锁的容器，保护里面的数据 | 共享状态一章 |

两点共性先记住：

- 它们都实现了 `Deref`，所以用起来 **就像直接在用里面的那个 `T`**，感知不到「隔着一层指针」。
- `*guard = fresh` 里的 `*` 是 **显式解引用**：往指针指向的那块内存整体赋一个新值。

----

# 其他高频语法

> 这些零碎但天天见的小语法，集中过一遍，省得在正文里反复解释。  
> `mut` / 遮蔽见 [《基础类型》](../lang/basics.md)；`if`/`for` 见 [《流程控制》](../lang/control-flow.md)。

```rust
// (1) 变量默认不可变；想改就加 mut
let mut n = 0;  n += 1;

// (2) shadowing（遮蔽）：用同名 let 重新绑定，旧的从此不可见（Go 里没有，别慌）
let t = Instant::now();
let t = Instant::now();   // 完全合法，而且常用

// (3) 块 {} 也是表达式：块里最后一个"不带分号的表达式"就是整个块的值
let (ver, items) = {
    let guard = data.read().unwrap();
    (guard.0, guard.1.clone())   // ← 没有分号 = 这就是块的返回值
};

// (4) 元组与解构
let pair = (1u64, "hi");
let (a, b) = pair;   // 解构：一次把两个字段拆到 a、b
let x = pair.0;      // 或者按位置取第 0 个

// (5) 元组结构体 + 「参数位置直接解构」（axum 的 handler 全靠它）
struct State<T>(T);
fn handler(State(state): State<AppState>) { /* ... */ }
//         ^^^^^^^^^^^^ 等价于先收进参数、再 let State(state) = arg;

// (6) 函数最后一行不带分号 = 返回值
fn add(a: i32, b: i32) -> i32 { a + b }

// (7) drop(x)：手动提前释放（书里用来提前关掉 channel 的发送端）
drop(tx);

// (8) 丢弃返回值
let _ = rx.await;                  // 显式丢弃，且不产生"未使用"警告
let _: () = con.set(k, v).await?;  // 丢弃 + 顺便用 : () 指定泛型返回类型
```

----

# Go→Rust 急救表

> 「这个 Go 写法在 Rust 里对应啥」——写码时最想随手查的那些，集中在这。更全的按主题查阅版见 [《Go → Rust 翻译词典》](../appendix/go-rust-dict.md)。

| 你想写的 Go | Rust 里怎么写 |
| --- | --- |
| `nil` | 没有！用 `Option<T>` 的 `None` |
| `if err != nil { return err }` | 一个 `?` |
| `defer mu.Unlock()` | 不用写：锁的 guard 离开作用域自动解锁（RAII） |
| `interface{}` / `any` | 尽量别用；优先泛型 `<T>`，或 `Box<dyn Trait>` |
| `go func(){...}()` | `tokio::spawn(async move {...})` |
| `chan T` | `tokio::sync::mpsc`（Tokio 一章） |
| struct 的方法 | 写在 `impl X { ... }` 块里 |
| `json:"name"` 标签 | `#[serde(rename = "name")]`（细讲见 [《JSON 序列化与反序列化》](../http/serde-json.md)） |
| `fmt.Sprintf` | `format!` |
| `panic` | `panic!` / `unwrap()`（仅限断言性场合） |

----

# 读完这篇之后

> 读到这，你已经有了读懂全书代码的语法储备。

某一章特有的新面孔（`Box::pin`、`ArcSwap`、`AsyncCommands`……），不在这篇里，  
去那一章文档末尾的「生词表」查。语法层面的疑问，随时翻回本篇对应小节。  
接下来建议先啃 [《所有权与借用》](../lang/ownership.md)，再读 [《生命周期》](../lang/lifetimes.md) ——那是全书的承重墙。
