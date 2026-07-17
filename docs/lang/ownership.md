# 为什么要有所有权

> 代码：[`code/lang-ownership/`](../../code/lang-ownership/)　  
> 运行：在 `code/` 目录下执行 `cargo run -p lang-ownership`

> 这是全书 **最重要** 的一篇，也是 Go 程序员劝退率最高的一关——因为 Go 有 GC，  
> 这套心智你从未需要过。别急着通读，每段代码都亲手编译一遍，报错也编一遍。

> 生命周期（`'a`、省略规则、`'static`）已拆到专章：[《生命周期》](lifetimes.md)。  
> 建议本篇吃透后再去；两篇合起来才是完整的「内存安全故事」。

堆上的内存用完必须归还。历史上有三种方案，各有各的代价：

| 方案 | 代表语言 | 代价 |
| --- | --- | --- |
| 程序员手动 `free` | C / C++ | 忘释放=泄漏；释放两次/释放后还用=崩溃或漏洞 |
| 垃圾回收器扫描 | Go / Java | 运行时开销、STW 停顿、内存占用偏高 |
| **编译期算出释放时机** | Rust | 学习成本（就是本篇 + 生命周期章） |

Rust 的思路是第三条：给每块数据指定一个 **唯一的责任人（所有者）**，  
责任人离开作用域的那一刻，编译器 **自动插入释放代码**。  
没有运行时、没有后台扫描——代价是你得遵守一套规则，让编译器能「算得出来」。

背后的操作系统事实（栈、堆、进程内存布局）见 [《操作系统基础》](../concurrency/os-basics.md)。

----

# 所有权三条规则

> 三句话，是整套系统的宪法。后面所有内容都是它们的推论。

1. 每个值有且只有一个 **所有者**（owner，通常是某个变量）；  
2. 所有者离开作用域，值被 **丢弃**（drop，自动释放）；  
3. 赋值 / 传参 / 返回值会把所有权 **移动**（move）给新变量——旧变量从此作废。

先记住这三条，下面逐条上代码验证。

----

# move 是交接不是拷贝

> Go 里 `b := a` 是「复制一份，两个都能用」。  
> Rust 里 `let b = a` 默认是「把责任交给 b，a 从此作废」。这是最颠覆直觉的一点。

```rust
let a = String::from("你好");   // a 是所有者
let b = a;                      // 所有权 move 给 b —— 不是拷贝！
// println!("{a}");             // ❌ error[E0382]: borrow of moved value: `a`
println!("{b}");                // ✅ 只有 b 能用
```

**move 在内存里做了什么**：`String` 在栈上是三个字——（指针，长度，容量）。  
`let b = a` 只把这三个字拷给 `b`，堆上字符数据 **原地不动**，再把 `a` 划为作废。  
所以 move 极其廉价：它是「责任交接」，不是「搬数据」。

**为什么必须作废 `a`**：若 `a`、`b` 都算活着，作用域结束会 double free。  
「作废旧变量」就是 Rust 对 double free 的编译期解法。

想两个都用？明确意图：

```rust
let c = String::from("苹果");
let d = c.clone();   // 深拷贝一份堆数据，两边各有主人
println!("{c} {d}"); // ✅
```

----

# 传参返回也是 move

> 函数参数、返回值走同一套规则：默认交接责任，不是「复制一份给函数」。

```rust
fn eat(s: String) { println!("吃掉 {s}"); }  // s 进函数后，结束时 drop

let a = String::from("香蕉");
eat(a);                 // 所有权 move 进函数
// println!("{a}");     // ❌ 已经交出去了
```

函数 **造出** 新数据时，用返回值把所有权交还给调用方——这是日常写法：

```rust
fn make_greeting(name: &str) -> String {
    format!("你好，{name}！")
}
let s = make_greeting("Gopher"); // s 成为新 String 的主人
```

对照 Go：`eat(a)` 复制头部/共享底层，两边随便用，GC 兜底。  
Rust 默认是交接——想共享，用借用，或 `Arc`（见 [《共享状态：Arc / RwLock》](../async/shared-state.md)）。

----

# Copy 类型是例外

> `i32` 这类「纯栈上、没有堆数据」的小类型，赋值按位复制，旧变量 **不作废** ——  
> 用起来和 Go 一样自然。

```rust
let x = 5;
let y = x;          // Copy，不是 move
println!("{x} {y}") // ✅
```

常见 `Copy`：整数、`bool`、`char`、浮点、以及「所有字段都是 Copy」的元组/小结构体。  
`Copy` 是标记 trait——本身没方法，只给编译器打标签。

判断口诀：**复制它，要不要动堆、要不要善后？**

- 不需要 → 可以是 `Copy`；  
- 需要（`String`/`Vec`）→ 不能 `Copy`，只能 move 或 `.clone()`。

重要规律：实现了 `Drop` 的类型 **不许** 再是 `Copy`。  
「要善后」和「随便复制」天生互斥——否则会善后多次（重复释放）。

----

# Drop 与 RAII

> 所有者离开作用域时，编译器自动「收尾」——释放堆、关文件、解锁。  
> 这套「资源生命跟着值走」叫 **RAII**，是 Rust 不需要 `defer` 的原因。

你会在后面几章反复见到它：

- [《共享状态：Arc / RwLock》](../async/shared-state.md)：锁 guard 离开作用域 = 自动解锁；  
- [《Tokio 运行时》](../async/tokio.md)：`drop(tx)` = 关闭发送端；  
- [《超时、限流与任务组》](../async/task-control.md)：`permit` 被 drop = 许可归还。

对照 Go：`defer` 挂在 **函数返回** 时统一执行；  
Rust 的 drop 是 **作用域级、跟着每一个值** ——`{}` 一结束就触发，且不可能忘写。

```rust
struct Guard(String);
impl Drop for Guard {
    fn drop(&mut self) { println!("{} 被释放", self.0); }
}

fn main() {
    let _a = Guard("外层".into());
    {
        let _b = Guard("内层".into());
    }                               // ← 「内层 被释放」
    println!("内层块已结束");
}                                   // ← 「外层 被释放」
```

drop 是 **后进先出**：越晚创建的越先释放。

----

# 借用不夺走所有权

> 每次都 move 太不方便——多数时候只想「看一眼」或「改一下」。  
> **借用**（引用）：借来用完就还，所有权始终在原主手里。

```rust
fn len(s: &String) -> usize { s.len() }      // 共享借用 &
fn shout(s: &mut String) { s.push('!'); }    // 可变借用 &mut

let mut a = String::from("你好");
println!("{}", len(&a));    // 借出去 → 用完 → a 仍是所有者
shout(&mut a);
println!("{a}");            // ✅
```

`&a` 只读，`&mut a` 可写。对应 Go「传指针用一下」，  
但 Rust 多了一条 Go 没有的铁律——下一节。

函数参数更常写 `&str` 而不是 `&String`（更能收字面量）——  
见 [《字符串、数组与切片》](strings-slices.md)。

----

# 多个读或一个写

> 借用的核心铁律：同一时刻，一个值 **要么被任意多人只读地借，要么只被一个人可写地借**，两者不可兼得。

```rust
let mut v = vec![1, 2, 3];
let r1 = &v;
let r2 = &v;             // ✅ 多个只读可共存
println!("{r1:?} {r2:?}");

let w = &mut v;          // ✅ 此时 r1/r2 已用完（见 NLL）
w.push(4);

let r3 = &v;
// v.push(5);            // ❌ E0502：已有只读借用，不能再可变借
println!("{r3:?}");
```

**为什么定这条规矩？**

- **防数据竞争**：「多读 XOR 一写」正是数据竞争的反面。  
  单线程就开始执行；多线程再配合 `Send`/`Sync`（[《共享状态：Arc / RwLock》](../async/shared-state.md)），  
  编译通过 ≈ 无数据竞争。Go 靠自觉加锁和 `-race`。  
- **防迭代器失效**：遍历时 `push` 可能扩容搬走数组，迭代指针悬空。  
  Go 里 `range` 时 `append` 合法但诡异；Rust 直接编译错误：

```rust
let mut v = vec![1, 2, 3];
for x in &v {            // 持有 &v
    // v.push(*x);       // ❌ E0502
    println!("{x}");
}
```

----

# 借用到最后一次使用

> 新手常以为「借用持续到 `{}` 结束」。其实更聪明：  
> 借用在 **最后一次被用到** 处就结束。这叫 **NLL**（non-lexical lifetimes）。

```rust
let mut s = String::from("hi");
let r = &s;
println!("{r}");     // r 最后一次使用 → 借用到此结束
let w = &mut s;      // ✅ 只读借用已「还了」
w.push('!');
```

主动技巧：用 `{}` 块把借用圈起来，让它提前结束——  
[《共享状态：Arc / RwLock》](../async/shared-state.md) 里「用块控制锁 guard」就是同款手法。

```rust
let mut data = vec![10, 20];
{
    let view = &data;
    println!("{view:?}");
}                    // view 结束
data.push(30);       // ✅
```

NLL 名字里有 lifetime，但管的是 **借用何时结束**；  
「返回的引用能不能活过参数」那本账，见 [《生命周期》](lifetimes.md)。

----

# 切片是一段视图

> `&v[1..3]`、`&s[..5]` 不是拷贝，而是「指向原数据某一段的借用」，叫切片。

```rust
let v = vec![1, 2, 3, 4, 5];
let mid: &[i64] = &v[1..4];     // [2,3,4] 的视图
let s = String::from("hello world");
let word: &str = &s[..5];       // "hello" 的视图
```

切片是胖指针（起点 + 长度），不复制数据。要点：

- `&str` 是 `String` 的切片视图，`&[T]` 是 `Vec` 的切片视图；  
- 切片是借用，受铁律管辖：持有期间原数据不可可变借用、不可被 move。

更深的字符串/UTF-8/数组切片见 [《字符串、数组与切片》](strings-slices.md)。

----

# Go 程序员碰壁清单

> 把 Go 习惯直接搬过来，最常撞的墙（生命周期相关见下一章）。

| 你在 Go 里的习惯 | 在 Rust 里撞的墙 | 改法 |
| --- | --- | --- |
| `b := a` 后两个都用 | E0382 | `.clone()` 或 `&a` |
| 随手塞进闭包/goroutine | E0373 / E0382 | `move`；共享用 `Arc`（[《函数与闭包》](functions-closures.md)） |
| 遍历时 `append` | E0502 | 循环外改；或索引 / `retain` |
| 到处传大 struct | 旧变量总作废 | 传 `&T`；多方持有用 `Arc` |
| 返回局部变量指针 | E0597 | 见 [《生命周期》](lifetimes.md)：改返回所有权 |

----

# 报错时的处置清单

> 借用检查器报错时，按成本从低到高试：

1. **调整顺序 / 缩小作用域**：利用 NLL，或用 `{}` 圈住借用；  
2. **拆函数**：少在一个函数里又读又写同一结构体；  
3. **`clone` 不可耻**：先跑通再优化；  
4. **共享所有权**：`Rc` / `Arc`（见 [《智能指针全家桶》](smart-pointers.md)）；  
5. **重新想归属**：一个所有者 + 消息传递（[《Tokio 运行时》](../async/tokio.md) 通道）。

----

# 动手实验

```bash
cd code
cargo run -p lang-ownership
```

1. 亲手撞 **E0382**（move 后使用）、 **E0502**（读写冲突）、 **E0499**（两个 `&mut`）；  
2. 造大 `Vec`，对比 `let b = a` 与 `.clone()` 的耗时——感受 move 有多便宜；  
3. 把 NLL 示例里 `println!("{r}")` 挪到 `w.push` 之后，看检查器翻脸；  
4. 读着本篇去跑 [《函数与闭包》](functions-closures.md) 的 `move` 示例。

生命周期相关实验（E0597、`longest`、`'static`）放到下一章做。

----

# 三句话带走

1. **所有权 = 唯一责任人，离开作用域自动 drop（RAII）**；  
   赋值/传参默认 move，Copy 类型例外。  
2. **借用 = 不接管责任地使用；铁律是「多读 XOR 一写」** ——  
   从单线程就消灭数据竞争和迭代器失效。  
3. **NLL 让借用活到最后一次使用**；  
   「引用能否比数据活得久」的账，下一章用生命周期说明书来算。

下一章：[《生命周期》](lifetimes.md)。若闭包捕获还糊，回看 [《函数与闭包》](functions-closures.md)。

----

# 附：本章生词表

- **move / 所有权转移**：责任人变更，旧变量作废；只拷栈上头部，堆数据不动。  
- **`Copy`**：赋值改为按位复制；与 `Drop` 互斥。  
- **`.clone()`**：显式深拷贝（对 `String`/`Vec` 会复制堆数据）。  
- **`Drop` / RAII**：离开作用域自动析构；资源生命跟着值走。  
- **`&T` / `&mut T`**：共享借用 / 可变借用；多读 XOR 一写。  
- **NLL**：借用在最后一次使用处结束，而非作用域末尾。  
- **切片 `&[T]` / `&str`**：指向连续数据一段的胖指针，是借用。  
- **借用检查器（borrow checker）**：编译期强制所有权与借用规则的部件。  
- **生命周期**：见专章——签名上的「引用有效期说明书」。
