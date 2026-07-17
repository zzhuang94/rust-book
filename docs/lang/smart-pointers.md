# 为什么这么多指针

> 代码：[`code/lang-smart-pointers/`](../../code/lang-smart-pointers/)　  
> 运行：在 `code/` 目录下执行 `cargo run -p lang-smart-pointers`

> 前置：[《所有权与借用》](ownership.md)、 [《类型系统与 trait》](types-traits.md)。  
> [《共享状态：Arc / RwLock》](../async/shared-state.md) 讲并发侧的 `Arc`/`Mutex`/`RwLock`；  
> 本篇把 **单线程侧** 兄弟讲透，并钉死 Go 转过来最容易踩的坑——  
> **没有 GC 之后，循环引用会真的泄漏**（`Weak` 的用武之地）。

[《所有权与借用》](ownership.md) 很严：一个值一个主人、共享只能借、借期间不能乱改。  
真实程序却总有「我就是要突破一下」的场景。  
每个智能指针，就是对某一条规则的 **受控豁免**：

| 我想要… | 单线程 | 多线程 | 豁免什么 |
| --- | --- | --- | --- |
| 值上堆 / 递归类型 / `dyn Trait` | `Box<T>` | 同 | （显式堆分配） |
| 多个所有者共享 | `Rc<T>` | `Arc<T>` | 「只有一个所有者」 |
| 只读手柄却要改内部 | `Cell` / `RefCell` | `Mutex` / `RwLock` | 「多读 XOR 一写」→ 运行期/锁 |
| 共享但不阻止释放 | `Weak<T>` | `Weak`（Arc 版） | 打破引用环 |
| 多半借用、偶尔拥有 | `Cow<'_, T>` | 同 | —— |

Go 对照总纲：**这些在 Go 里「都不存在」** ——指针随便复制、随便改，GC 收尸。  
Rust 把「怎么共享」写成类型，成本和约束一目了然。

----

# Box 就是放堆上

> 最朴素的智能指针：把值放到堆上，`Box` 是唯一所有者。  
> 用起来像直接用里面的值（`Deref` 自动解引用）。

```rust
let b = Box::new(5);     // 5 在堆上，b 拥有它
println!("{}", *b);      // 显式解引用；多数时候连 * 都不用写
```

对照 Go：栈还是堆由逃逸分析决定，你无感。  
Rust 默认偏栈， **上堆要显式 `Box`** ——想清楚、写清楚。

----

# Box 的三大用途

> 单独「就为了上堆」不多见；真实价值在下面三件事。

**1. 递归类型** ——节点里若直接嵌自己，类型大小无限，编译不过：

```rust
enum Tree {
    Leaf(i64),
    Node(Box<Tree>, Box<Tree>),  // 不加 Box → E0072 infinite size
}
```

`Box` 把「里边还有一棵树」变成「一个指针那么大」，尺寸算得出来。

**2. trait 对象载体** ——异构集合：

```rust
let crowd: Vec<Box<dyn Greeter>> = vec![
    Box::new(En),
    Box::new(Zh),
];
```

不同具体类型，统一当 `dyn Greeter` 用（见 [《类型系统与 trait》](types-traits.md)）。

**3. 减小 move 成本** ——巨型结构体 move 会拷整块栈数据；  
包进 `Box` 后，move 只拷一个指针。

----

# Deref 让指针变透明

> 为什么 `Box<String>` 能直接 `.len()`？因为实现了 **`Deref`**。  
> 编译器会一层层自动解引用（deref coercion）：

```rust
let b = Box::new(String::from("hello"));
takes_str(&b);   // &Box<String> → &String → &str
```

`Rc`、`Arc`、`Cow` 同理——所以智能指针用起来不像「隔着一层」。  
⚠️ **别用 `Deref` 模拟继承**：它是给智能指针透明化用的，不是给业务类型偷方法用的。

`Drop` 则负责善后：`Box` 释放堆、`Rc` 减计数、锁 guard 解锁——  
见 [《所有权与借用》](ownership.md) 的 RAII。

----

# Rc 单线程共享

> `Rc<T>` = Reference Counted：`clone` 计数 +1，`drop` -1，归零才释放。  
> 数据本身不复制——多个变量共同「拥有」同一份堆数据。

```rust
use std::rc::Rc;

let a = Rc::new(String::from("共享数据"));
let b = Rc::clone(&a);    // 习惯写 Rc::clone(&a)，强调「加计数」不是深拷贝
println!("strong={}", Rc::strong_count(&a));
```

和 `Arc` 的唯一关键区别：**计数不是原子的** → 更快，但没有 `Send`/`Sync`，  
**不能跨线程**。口诀：

- 单线程、图快 → `Rc`；  
- 跨线程 / `tokio::spawn` → `Arc`；  
- 拿不准 → **用 `Arc`**（顶多慢一点，绝不会错）。

`Rc<T>` 给你的仍是 **只读** 视图（多个所有者，仍守「共享不可变」）。  
想改 → 下一节内部可变性。

----

# Cell 整存整取

> **内部可变性**：外面只有 `&T`（只读手柄），里面却能改。  
> `Cell<T>` 适合 **`T: Copy` 的小值**（计数、开关）。

```rust
use std::cell::Cell;

let c = Cell::new(0);
c.set(c.get() + 1);   // 通过 &c 就能改
```

特点：进出都是拷贝， **不产生借用** → 永不因借用规则 panic，运行时零额外检查。  
不能装 `String`/`Vec`（不是 Copy）——那些请用 `RefCell`。

----

# RefCell 运行期借检查

> `RefCell<T>` 把「多读 XOR 一写」从 **编译期** 挪到 **运行期**。  
> 适合非 Copy 的值（`Vec`、自定义结构体）。

```rust
use std::cell::RefCell;

let cell = RefCell::new(vec![1, 2, 3]);
{
    let mut w = cell.borrow_mut();
    w.push(4);
}                              // w 结束 → 写借用自动归还
println!("{:?}", cell.borrow());
```

违规不会编译失败，而是 **运行时 panic**：

```rust
let _r = cell.borrow();
// let _w = cell.borrow_mut();  // 💥 already borrowed: BorrowMutError
```

**核心交易**：借用规则一条不少，检查推迟到运行期——  
代价从「编不过」变成「跑起来崩」。  
所以 `RefCell` 是 **后门 / 最后手段**：用在「你确知安全、但编译器看不懂」的共享模式上，  
不是日常默认选项。

----

# Rc 配 RefCell

> `Rc<RefCell<T>>` = 单线程「共享 + 可改」。  
> 和 [《共享状态：Arc / RwLock》](../async/shared-state.md) 的 `Arc<Mutex<T>>` **同一张图纸**：

| 层 | 单线程 | 多线程 |
| --- | --- | --- |
| 共享所有权 | `Rc` | `Arc` |
| 内部可变性 | `RefCell`（违规 panic） | `Mutex`/`RwLock`（排队） |
| 组合 | `Rc<RefCell<T>>` | `Arc<Mutex<T>>` |

```rust
let shared = Rc::new(RefCell::new(vec![1, 2, 3]));
let also = Rc::clone(&shared);
also.borrow_mut().push(4);
println!("{:?}", shared.borrow());  // [1,2,3,4]
```

服务端正文多是右列；CLI、单线程图、解释器环境里左列很常见。

----

# 引用环会泄漏

> **本篇重点。**  
> Go 的 GC 做可达性分析，环不是问题；  
> Rust 的引用计数只看「还有几个人拿着」—— **环里的对象互相拿着，永远不归零**。

错误结构（父子都用 `Rc` 回指）：

```rust
struct LeakyNode {
    children: RefCell<Vec<Rc<LeakyNode>>>,
    parent: RefCell<Option<Rc<LeakyNode>>>,  // ❌ 强回指
}
```

外界把 `parent`/`child` 变量都丢了之后：  
父计数 ≥1（被子指着），子计数 ≥1（被父指着）→ **Drop 永不执行**。  
这是安全 Rust 里少数「能编译、会泄漏」的写法——  
Go 转过来特别容易中招，因为你从未想过「谁指着谁会不会成环」。

跑示例里的 `demo_cycle_leaks`：块结束后 **看不到** Drop 打印，就是泄漏现场。

----

# Weak 打破环

> 解法：回指改成 **`Weak`** ——「不算数的引用」：  
> 不加强计数、不阻止释放；用时 `upgrade()` 试着换回 `Rc`。

```rust
struct Node {
    children: RefCell<Vec<Rc<Node>>>,   // 拥有方向：强
    parent: RefCell<Weak<Node>>,        // 回指方向：弱
}

let parent = Rc::new(...);
let child = Rc::new(Node {
    parent: RefCell::new(Rc::downgrade(&parent)),
    ...
});
```

**设计口诀**：先想清「谁拥有谁」——  
**拥有用强引用（`Rc`/`Arc`），回指用 `Weak`**。

常见成环现场：

- 树：父拥有子，子回望父；  
- 缓存：缓存拥有条目，条目回望缓存；  
- 观察者 / 回调表：注册表拥有对象，回调又捕获对象（服务端高发）。

多线程同理：`Arc` + `std::sync::Weak`。

----

# upgrade 返回 Option

> `weak.upgrade()` → `Option<Rc<T>>`：还活着是 `Some`，已释放是 `None`。  
> 逼你面对「它可能已经没了」——把 Go 里隐性的僵尸对象 bug 显性化。

```rust
if let Some(p) = node.parent.borrow().upgrade() {
    println!("父还在，value={}", p.value);
} else {
    println!("父已经不在了");
}
```

示例 `demo_upgrade_after_drop`：父 `Rc` 丢光之后，手里的 `Weak` 升级得到 `None`，  
对象该释放就释放——不会因为「还有人弱引用着」而假活。

----

# Cow 借用或拥有

> `Cow<'_, T>`（Clone-on-Write）：  
> 函数 **多数时候** 原样返回输入（想零拷贝）， **偶尔** 才要改（必须拥有）。

```rust
use std::borrow::Cow;

fn normalize(input: &str) -> Cow<'_, str> {
    if input.contains(' ') {
        Cow::Owned(input.replace(' ', "_"))
    } else {
        Cow::Borrowed(input)
    }
}
```

- 返回 `&str`：偶尔要改时做不到；  
- 返回 `String`：多数不改也白白分配。  
`Cow` 两全其美；调用方当 `&str` 用即可（自动 `Deref`）。

热路径上的规整化、转义、解码很常见；读代码能认即可。

----

# 和 Arc Mutex 怎么对照

> 把整张地图收一口：

```
上堆 / 递归 / dyn Trait ──────────────→ Box<T>

多个所有者？
├─ 单线程 → Rc<T>  ─┬─ 只读
│                   └─ 要改 → Rc<RefCell<T>>
└─ 跨线程 → Arc<T> ─┬─ 只读
                    └─ 要改 → Arc<Mutex<T>> / Arc<RwLock<T>>

有回指 / 观察者？ → 那条边用 Weak（Rc 或 Arc 版）

多半借、偶尔改？ → Cow<'_, T>
Copy 小值内部可变？ → Cell<T>
```

本书异步主线几乎全是右列（`Arc` + 锁）；  
本篇左列是读懂单线程生态、以及理解右列「为什么长这样」的钥匙。

----

# 常见踩坑

1. **`Rc::clone` 写成 `a.clone()`** ——能跑，但读者以为你在深拷贝 `String`；  
   团队习惯写 `Rc::clone(&a)` / `Arc::clone(&a)`。  
2. **该 `Arc` 用了 `Rc`** ——一 `spawn` 编译器就拦；听它的。  
3. **`RefCell` 跨 `.await` 持有 `borrow_mut` guard** ——异步里大忌；  
   跨 await 用 `tokio::sync::Mutex`（见共享状态章）。  
4. **忘记 Weak** ——树/回调表默默泄漏，还没有编译错误。给节点实现 `Drop` 打日志验货。  
5. **用 `Deref` 搞「继承」** ——别；要复用行为用 trait。

----

# 动手实验

```bash
cd code
cargo run -p lang-smart-pointers
```

1. 看 `demo_cycle_leaks`：**没有** Drop 打印 = 泄漏；再对比 `demo_weak_fixes_cycle`；  
2. 取消注释 RefCell 双借，体验 `BorrowMutError`；  
3. 打印一棵小树各阶段的 `strong_count` / `weak_count`；  
4. 去掉 `Tree` 里的 `Box`，读 E0072；  
5. 用 `matches!(..., Cow::Borrowed(_))` 验证无空格路径零拷贝。

----

# 三句话带走

1. **智能指针 = 对所有权规则的受控豁免**，成本和约束写在类型上。  
2. **`Rc<RefCell<T>>` ↔ `Arc<Mutex<T>>`** 是同一图纸的单/多线程版；  
   `RefCell` 是后门，违规就 panic。  
3. **引用计数收不了环**：拥有用强、回指用弱；`upgrade` 的 `None` 消灭僵尸对象。

下一章：[《通用错误处理》](error-handling.md)。  
并发共享接着读 [《共享状态：Arc / RwLock》](../async/shared-state.md)。

----

# 附：本章生词表

- **`Box<T>`**：堆上唯一所有者；递归、`dyn Trait`、减 move 成本。  
- **`Rc<T>` / `Arc<T>`**：单线程 / 多线程引用计数共享。  
- **`Rc::clone` / `strong_count` / `weak_count`**：加强计数；查看强/弱计数。  
- **`Cell<T>`**：Copy 小值内部可变；get/set，无借用 panic。  
- **`RefCell<T>` / `borrow` / `borrow_mut`**：运行期借用检查；违规 `BorrowMutError`。  
- **内部可变性**：通过 `&` 修改内部数据的能力。  
- **`Weak<T>` / `downgrade` / `upgrade`**：弱引用；降级与升级；破环防泄漏。  
- **引用环**：对象互相强引用导致计数永不归零。  
- **`Cow<'a, T>`**：`Borrowed` | `Owned`；写时克隆。  
- **`Deref` / deref coercion**：自动解引用，让智能指针用法透明。  
- **`Drop`**：离开作用域时的析构钩子（RAII）。
