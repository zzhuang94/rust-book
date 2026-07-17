# 这张地图怎么用

> 你已经有一整套 Go 的心智模型：值和指针、`err != nil`、goroutine、GC。这一章不教新语法，  
> 而是把你脑子里这套模型，一条条对到 Rust 上，并标出「看着像、其实不一样」的坑。

读法很简单：先扫一遍下一节的总表建立全局印象，然后每个「差异点」都配了最小代码和「Go 里怎样、Rust 里怎样、  
为什么不同」。真正要动手深挖某个主题时，跟着每节末尾的链接跳到对应章。

一句话定基调：**Go 的设计哲学是「运行时帮你兜底」（GC、调度器、简单类型系统），Rust 的哲学是「编译期把话说死」（所有权、  
无 GC、强类型）**。你遇到的绝大多数「Rust 怎么这么麻烦」，根子都在这句话——它把 Go 留到运行时的问题，  
提前到编译期让你解决。

----

# 一眼看尽差异

> 这张表是全书的索引。现在看不懂没关系，每一行后面都有对应的章节展开。

| 维度 | Go | Rust |
| --- | --- | --- |
| 内存管理 | GC 自动回收 | 所有权 + 编译期插入释放，无 GC |
| 变量默认 | 可变 | **不可变**，要改写 `mut` |
| 赋值/传参 | 拷贝值或共享引用 | **移动** 所有权（或显式借用） |
| 空值 | `nil` | 没有 nil，用 `Option<T>` |
| 错误 | `(v, err)` 多返回值 | `Result<T, E>` + `?` |
| 异常 | `panic`/`recover` | `panic!`（仅用于不可恢复），业务错误走 `Result` |
| 并发 | `go`、channel 语言内建 | `async/await` 语法 + 外部运行时 Tokio |
| 接口 | 隐式满足 | `trait`，必须 `impl X for Y` 显式实现 |
| 泛型 | 1.18 起，较克制 | 语言核心，单态化 |
| 零值 | 每种类型都有零值 | 没有零值概念，必须显式初始化 |
| 可见性 | 首字母大小写 | `pub` 关键字 |
| 构建 | `go build` | `cargo build`（多一层 crate/workspace） |

下面挑其中差异最大、最容易绊倒 Go 程序员的几条，逐个上代码。

----

# 值默认被移动

> 这是 Go 转 Rust 的 **头号震撼**：在 Rust 里，把一个值赋给别人、或传进函数，默认会把它「搬走」——原来的名字 **就不能再用了**。

先看 Go。Go 里赋值/传参是「拷贝」，原变量照常可用：

```go
// Go：s2 是 s 的一份拷贝（底层数组共享，但变量本身各用各的）
s := []int{1, 2, 3}
s2 := s
fmt.Println(s)   // ✅ 照常能用
```

同样的写法在 Rust 里，`v` 被「移动」给了 `v2`，之后再用 `v` 直接 **编译不过**：

```rust
let v = vec![1, 2, 3];
let v2 = v;            // v 的所有权移动给 v2
// println!("{:?}", v);  // ❌ 编译错误：borrow of moved value: `v`
println!("{:?}", v2);    // ✅ 只有 v2 能用
```

为什么 Rust 要这么设计？因为它没有 GC。一份数据在同一时刻只能有 **一个所有者**，所有者离开作用域时负责释放它。  
如果 `v` 和 `v2` 都算「拥有」，释放时就会 double free。移动语义保证「同一份数据只有一个主人」，  
从根上杜绝了这类问题。

想「两个都能用」，你有两个选择，对应两种意图：

```rust
let v = vec![1, 2, 3];
let v2 = v.clone();      // (1) 深拷贝一份，各拥有各的（对应 Go 的显式 copy）
println!("{:?} {:?}", v, v2);   // 两个都能用

let v3 = vec![1, 2, 3];
let borrow = &v3;        // (2) 借用：只是"借看一眼"，不夺走所有权
println!("{:?} {:?}", v3, borrow);  // 两个都能用
```

这套「移动 / 借用 / 克隆」是 Rust 的地基，见 [《所有权与借用》](../lang/ownership.md)；  
引用活多久见 [《生命周期》](../lang/lifetimes.md)。  
它是全书最该花时间的一篇。

----

# 错误是值不是异常

> Go 已经把你训练得很好：错误是返回值，`if err != nil` 满天飞。Rust 走得更彻底——错误被塞进一个类型里，  
> 你 **不处理就拿不到成功的值**。

Go 的经典写法：

```go
f, err := os.Open("x.txt")
if err != nil {
    return err          // 一层层往上抛
}
// 用 f...
```

Rust 里，`open` 返回的是 `Result<File, Error>`——要么 `Ok(file)`，  
要么 `Err(e)`。那个 `?` 就是「成功就取出 file，失败就把错误 return 出去」，一个字符顶 Go 的三行：

```rust
fn read() -> Result<String, std::io::Error> {
    let mut f = std::fs::File::open("x.txt")?;  // ? = 失败就 return Err
    let mut s = String::new();
    f.read_to_string(&mut s)?;                  // 又一个 ?
    Ok(s)                                       // 成功用 Ok 包起来返回
}
```

关键差别，也是 Rust 更安全的地方：Go 里你 **可以** 忽略 `err` 直接用 `f`（编译器不拦你，运行时才炸）；  
Rust 里成功值 `file` 被 **包在 `Ok` 里**，不「拆开」`Result` 就根本拿不到它——「拿了返回值忘了看错误」这种 bug 在 Rust 里不可能发生。

至于 Go 的 `panic`/`recover`：Rust 也有 `panic!`，但文化上 **只用于「程序员断言这绝不该发生」**，  
比如数组越界、`unwrap()` 一个明知是 `Some` 的值。可恢复的业务错误一律走 `Result`。详见 [《通用错误处理》](../lang/error-handling.md)。

----

# 没有 GC 靠所有权

> Go 有 GC，你共享数据从不操心谁来释放。Rust 没有 GC——它靠「所有权 + 作用域」在编译期就决定每块内存何时释放，一个运行时开销都不留。

在 Go 里，多个 goroutine 共享一个指针，GC 会在没人用时回收它，你什么都不用管：

```go
data := &Config{...}
go use(data)     // 随便共享，GC 兜底
go use(data)
```

Rust 没有 GC，「多个地方共享同一份数据」得用 `Arc`（原子引用计数）显式表达——它记录「现在有几个人拿着」，减到 0 就释放：

```rust
use std::sync::Arc;

let data = Arc::new(Config { /* ... */ });
let d1 = Arc::clone(&data);   // 计数 +1，d1、data 指向同一份
let d2 = Arc::clone(&data);   // 计数 +1
// 每个 dN 离开作用域，计数 -1；减到 0，Config 被释放。全程无 GC 扫描。
```

看起来比 Go 啰嗦，但换来的是：**没有 GC 停顿、内存何时释放完全确定**。这对延迟敏感的服务是实打实的好处。  
`Arc` 是全书高并发部分的主角， [《智能指针全家桶》](../lang/smart-pointers.md) 和 [《共享状态：Arc / RwLock》](../async/shared-state.md) 会细讲。

----

# 并发是库不是关键字

> Go 把并发做进了语言：`go` 一个关键字就起一个 goroutine，channel 是内建类型，运行时（调度器）默认就在。  
> Rust 只提供 `async/await` 语法， **运行时要你自己引** ——这就是 Tokio 的位置。

Go 里并发是「语言自带电池」：

```go
go worker()                 // 关键字起协程
ch := make(chan int, 10)    // 内建 channel
```

Rust 里，`async fn` 只是定义了一个「可暂停的状态机」，它 **不会自己跑**；你得把它交给一个执行器（executor）。  
全书用的执行器是 Tokio：

```rust
#[tokio::main]                          // 引入 Tokio 运行时（≈ 手动装上 Go 的调度器）
async fn main() {
    tokio::spawn(async { worker().await });          // ≈ go worker()
    let (tx, rx) = tokio::sync::mpsc::channel(10);    // ≈ make(chan, 10)
}
```

一句话记住：**Tokio ≈ 你手动引入的、可插拔的 Go 调度器**。为什么 Rust 不像 Go 那样内建？  
因为 Rust 要覆盖从嵌入式到服务器的场景，把运行时做进语言会太重、太死；拆出来反而灵活。这套模型是全书主线，  
从 [《async 基础》](../async/basics.md) 一路展开。

----

# 可变性必须显式

> Go 里变量天生可改。Rust 反过来：变量 **默认不可变**，想改必须写 `mut`；连「借出去让别人改」也要专门的 `&mut`。

```rust
let x = 5;
// x = 6;          // ❌ 编译错误：cannot assign twice to immutable variable
let mut y = 5;     // 加 mut 才能改
y = 6;             // ✅

fn add_one(n: &mut i32) {   // 收一个"可变借用"，才能改动调用方的变量
    *n += 1;                // * 解引用后修改
}
```

这不是找茬。默认不可变让「这个值会不会被改」在代码里一目了然，也是 Rust 并发安全的基石之一：编译器有一条铁律——**同一时刻，  
要么有多个只读借用，要么只有一个可变借用，二者不可兼得**。这条规则在编译期就消灭了数据竞争（data race），  
而 Go 得靠你自觉加锁、靠 `-race` 检测器碰运气。展开见 [《所有权与借用》](../lang/ownership.md)。

----

# 接口要显式实现

> Go 的接口是「隐式」的：类型凑齐方法就自动满足。Rust 的 `trait` 必须「显式」写 `impl Trait for Type`，谁实现了谁，白纸黑字。

Go：

```go
type Stringer interface { String() string }
type User struct{ name string }
func (u User) String() string { return u.name }  // 没写"实现 Stringer"，但自动满足
```

Rust：

```rust
trait Stringer { fn to_str(&self) -> String; }
struct User { name: String }

impl Stringer for User {                 // 必须显式声明"User 实现 Stringer"
    fn to_str(&self) -> String { self.name.clone() }
}
```

代价是啰嗦一点，好处是「这个类型到底实现了哪些 trait」永远清清楚楚，IDE 也能精确跳转。还有一个 Go 程序员必踩的坑：  
**trait 的方法要先 `use` 进作用域才能调用**（比如用 redis 时得先 `use redis::AsyncCommands;`，  
连接对象才「长出」`.get()`）。trait 是 Rust 抽象的核心，见 [《类型系统与 trait》](../lang/types-traits.md)。

----

# 生态选型对照

> 换了语言，工具箱也得换。这张表把你常用的 Go 库/包，对到 Rust 生态里最主流的选择，少走选型弯路。

| 你在 Go 用 | Rust 里对应 | 本书哪一章 |
| --- | --- | --- |
| `net/http` / Gin | axum（基于 hyper/tokio） | [《axum 入门》](../http/axum.md) |
| `encoding/json` | serde + serde_json | 贯穿全书 |
| `database/sql` / GORM | sqlx | [《sqlx 数据库》](../http/sqlx.md) |
| `go-redis` | redis-rs | [《接入 Redis》](../http/redis.md) |
| `context.Context`（取消/超时） | `CancellationToken` / `tokio::time::timeout` | [《超时、限流与任务组》](../async/task-control.md) |
| `sync.Mutex` / `RWMutex` | `std::sync::Mutex` / `RwLock` | [《共享状态：Arc / RwLock》](../async/shared-state.md) |
| `sync/atomic` | `std::sync::atomic` | [《共享状态：Arc / RwLock》](../async/shared-state.md) |
| `log` / `zap` | `tracing` | [《tracing 结构化日志》](../engineering/tracing.md) |
| `errors` / `fmt.Errorf` | `thiserror`（库）/ `anyhow`（应用） | [《通用错误处理》](../lang/error-handling.md) |
| `time` | `chrono` / `std::time` | 贯穿全书 |
| 内建 testing | 内建 `#[test]` + `#[tokio::test]` | [《测试》](../engineering/testing.md) |

----

# 该读哪一章

> 差异地图看完，给你一条按「痛点优先级」排的深挖路线。

- **最该先啃**：[《所有权与借用》](../lang/ownership.md) + [《生命周期》](../lang/lifetimes.md)。  
  上面「移动 / 借用 / 可变性 / 无 GC」的震撼全在这解决，  
  它是全书承重墙。
- **紧接着**：[《类型系统与 trait》](../lang/types-traits.md) + [《通用错误处理》](../lang/error-handling.md)，  
  把「接口」和「错误」两块 Go 直觉平移过来。
- **进主线前**：如果对 `mod`/`pub`/crate 组织还发怵，插一篇 [《模块、crate 与可见性》](../lang/modules.md)。
- **然后进异步主线**：[《async 基础》](../async/basics.md) → [《Tokio 运行时》](../async/tokio.md) → 共享状态 → HTTP 服务。
- **随手查**：写码时「这个 Go 写法对应啥」，翻 [《Go → Rust 翻译词典》](../appendix/go-rust-dict.md)。

----

# 附：本章生词表

按出现顺序，解释本章第一次露面的名词：

- **移动（move）**：把一个值的所有权从一个变量转给另一个，原变量随后失效。Rust 赋值/传参的默认行为。
- **所有权（ownership）**：Rust 的核心规则——每份数据在同一时刻只有一个「所有者」，所有者离开作用域时负责释放它。
- **借用（borrow）**：用 `&`（只读）或 `&mut`（可变）临时「借看/借改」一个值，不夺走所有权。
- **克隆（clone）**：显式深拷贝一份数据，得到互不影响的两份。对应 Go 里手动 copy。
- **`Arc`（原子引用计数）**：让一份数据被多处共享、并在最后一个持有者离开时释放的智能指针；可跨线程。
- **执行器 / 运行时（executor / runtime）**：真正驱动 `async` 状态机运行的东西。Rust 不内建，靠 Tokio 提供。
- **`mut` / `&mut`**：`mut` 让变量可改；`&mut` 是「可变借用」，允许借用方修改原值。
- **数据竞争（data race）**：多个线程无同步地并发读写同一数据，结果不确定。Rust 靠借用规则在编译期消除它。
- **trait**：Rust 的接口机制，但需 `impl Trait for Type` 显式实现。
- **serde**：Rust 事实标准的序列化/反序列化框架，`#[derive(Serialize)]` 一贴即用。
