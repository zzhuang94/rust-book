# 函数与闭包

> 代码：[`code/lang-functions-closures/`](../../code/lang-functions-closures/)　  
> 运行：在 `code/` 目录下执行 `cargo run -p lang-functions-closures`

> 前置：[《所有权与借用》](ownership.md)（必读）、 [《流程控制》](control-flow.md)。  
> 函数长得和 Go 像；闭包也像匿名函数。真正难的是：  
> **闭包怎么捕获外面的变量** ——没有 GC 之后，这就是 move / 借用规则的投影。  
> 全书满天飞的 `async move { ... }`，根子都在这一章。

----

# 函数长什么样

> 参数类型在 `:` 后面，返回类型用 `->`。对照 Go 只是标点不同。

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn greet(name: &str) {          // 无 -> 就是返回 unit ()
    println!("你好，{name}！");
}
```

| | Go | Rust |
| --- | --- | --- |
| 声明 | `func add(a, b int) int` | `fn add(a: i32, b: i32) -> i32` |
| 类型位置 | 名后、无冒号 | 名后、 **有冒号** |
| 多返回 | `func f() (int, error)` | 返回元组 `(i32, Error)` 或 `Result` |

函数名、参数名默认在当前模块内可见；跨模块要 `pub fn`——见 [《模块、crate 与可见性》](modules.md)。

----

# 无分号才是返回

> Rust 函数体最后一个 **不带分号的表达式** 就是返回值。  
> 这是 Go 程序员最容易愣住的点之一（[《环境与工具链》](../start/toolchain.md) 里的 hello 已剧透过）。

```rust
fn answer_ok() -> i32 {
    42          // ✅ 返回 42
}

fn answer_bad() -> i32 {
    42;         // ❌ 变成语句，函数实际返回 ()，与 -> i32 冲突
}
```

需要 **提前离开** 时，仍用 `return`（带不带分号按语句规则）：

```rust
fn early(flag: bool) -> i32 {
    if flag {
        return 1;
    }
    0
}
```

口诀：**正常路径靠最后一行表达式；中途逃跑用 `return`。**

----

# 签名就是传参契约

> 参数写成 `T`、`&T` 还是 `&mut T`，直接决定：  
> 调用方是交出所有权、只读借出，还是借出可改权。  
> 对照 Go：全是「拷贝或共享，靠你自觉不乱改」；Rust 写在类型上。

```rust
fn print_len(s: &String) { /* 只看 */ }
fn bump(n: &mut i32) { *n += 1; }   // 要改，就得 &mut
fn eat(s: String) { /* 吃掉所有权 */ }

let name = String::from("Rust");
print_len(&name);     // 借一下，name 还在
// eat(name);         // 若调用，name 此后作废
```

经验法则（和所有权章同一套）：

1. 只读查看 → `&T`（字符串场景常写更通用的 `&str`）；  
2. 要修改调用方的值 → `&mut T`；  
3. 函数结束后调用方不再需要 → 直接 `T`（move 进来）。

----

# 闭包是能捕获的匿名函数

> 闭包 ≈ Go 的 `func(a int) int { return a + 1 }`。  
> 语法：`|参数| 表达式`，也可以 `|参数| { 多行; 值 }`。

```rust
let add = |a: i32, b: i32| a + b;
let add_infer = |a, b| a + b;      // 类型由第一次调用推断，之后固定
let describe = |n: i32| {
    let kind = if n % 2 == 0 { "偶" } else { "奇" };
    format!("{n} 是{kind}数")      // 块的最后表达式 = 返回值
};
```

和 `fn` 的关键差别：**闭包可以「看见」定义处外层的局部变量**（捕获）。  
`fn` 做不到——它只能碰参数、全局和常量。

----

# 默认捕获是借用

> 编译器能借就不抢：闭包默认用 **不可变借用** 抓住外层变量。  
> 外面的变量在闭包用完后往往还能继续用。

```rust
let prefix = String::from("日志");
let log = |msg: &str| {
    println!("[{prefix}] {msg}");  // 捕获 &prefix
};
log("启动");
println!("{prefix}");              // ✅ prefix 仍在
```

对照 Go：闭包捕获变量通常是引用语义，外加 GC 保证活着。  
Rust 同样先借，但 **借多久、能不能再借**，全归借用检查器管。

----

# 要改环境就可变捕获

> 闭包里若修改了外层变量，它会对该变量做 **`&mut` 捕获**。  
> 外层变量必须是 `mut`，且在闭包存活期间，别人不能再借这个变量。

```rust
let mut count = 0;
let mut tick = || {
    count += 1;
};
tick();
tick();
```

注意：`tick` 本身也要 `let mut tick`——因为调用 `FnMut` 闭包会改它的内部状态  
（捕获的可变借用算状态的一部分）。

----

# move 把所有权搬进去

> 写上 `move`，捕获改为 **夺取所有权**（Copy 类型则是拷进闭包）。  
> 外面不能再碰这些变量——这正是「把任务丢到别处跑」时要的。

```rust
let name = String::from("异步任务");
let task = move || {
    println!("任务持有：{name}");
};
task();
// println!("{name}");   // ❌ name 已经搬进 task
```

什么时候必须 `move`？

- 闭包（或 `async` 块）可能活得 **比当前函数更久**；  
- 最典型：`tokio::spawn(async move { ... })`、把闭包存进结构体、交给别的线程。

若不 `move`、只借栈上的局部变量：函数返回 → 栈没了 → 悬垂引用 → **编译器直接拒绝**。  
Go 靠 GC 让你「随手捕获」；Rust 用 `move` + 所有权在编译期做同一件事。

----

# 三种 Fn 特质

> 闭包不是「一种类型」，而是自动实现以下特质（之一或更多）：

| 特质 | 能调用几次 | 捕获方式直觉 | 典型场景 |
| --- | --- | --- | --- |
| `Fn` | 无限次 | 只不可变借（或无捕获） | 纯回调、多次 map |
| `FnMut` | 无限次 | 可能可变借 | 累加器、带状态的回调 |
| `FnOnce` | **一次** | 消费所有权（或只调用一次） | `move` 吃掉了 `String` 等 |

关系（简化）：能 `Fn` 的也能当 `FnMut` / `FnOnce` 用；  
能 `FnMut` 的也能当 `FnOnce` 用。 **要求越松，能接受的闭包越多。**

```rust
fn call_twice<F>(f: &F)
where
    F: Fn(),           // 要求：可反复、只读
{
    f();
    f();
}
```

你若写了一个 `move || drop(token)` 且 `token: String`，它往往只实现 `FnOnce`——  
调用第二次会编译不过：所有权已经在第一次里被吃掉了。

> 🔬 **底层直觉**：闭包编译后是一个 **匿名结构体**，捕获的变量变成字段；  
> `Fn*` 就是在这个结构体上调用的方法（`call` / `call_mut` / `call_once`）。  
> `move` 与否决定字段是「引用」还是「拥有的值」。

----

# 闭包当回调传进去

> 标准库到处是闭包：`map`、`filter`、`unwrap_or_else`…  
> 自己写「高阶函数」时：参数用泛型 + `Fn` / `FnMut` / `FnOnce` 约束。

```rust
let doubled: Vec<i32> = nums.iter().map(|x| x * 2).collect();

fn apply_twice<F>(x: i32, f: F) -> i32
where
    F: Fn(i32) -> i32,
{
    f(f(x))
}

let sum = apply_twice(5, |x| x + 3);   // f(f(5)) → 5+3=8，再 +3 → 11
```


对照 Go：`func Apply(x int, f func(int) int) int`——一样是「函数当参数」。  
Rust 多了 `Fn`/`FnMut`/`FnOnce` 三档，好把「能不能调用两次」写进类型。

选型口诀：

- 回调里 **不改** 环境、可能调多次 → `Fn`；  
- 要改环境、调多次 → `FnMut`；  
- 只保证调一次（或必须吃掉所有权）→ `FnOnce`。

----

# 和 async move 的关系

> 你后面会写无数次：

```rust
tokio::spawn(async move {
    do_work(name).await;
});
```

拆开看：

1. `async { ... }` 变成一个实现了 `Future` 的状态机（[《async 基础》](../async/basics.md)）；  
2. **`move`** 把 `name` 等捕获变量的所有权搬进这个 Future；  
3. `spawn` 把 Future 丢到运行时，可能在别的任务/线程上跑——  
   此时当前函数栈可能早已返回，所以 **只能拥有，不能再借栈上的东西**。

本章的 `move || { ... }` 和 `async move { ... }` 是 **同一类捕获问题**；  
只是一个立刻可调用，一个要 `.await` / 执行器推动。

同步模拟「稍后再跑」：

```rust
fn run_later<F: FnOnce()>(job: F) {
    job();
}

let name = String::from("worker");
run_later(move || println!("{name}"));
```

----

# 常见踩坑

**1. 闭包推断后类型锁死**

```rust
let f = |x| x;
f(1);
// f("hi");   // ❌ 第一次调用已把 x 钉成 i32
```

**2. 可变捕获时又想用原变量**

```rust
let mut n = 0;
let mut inc = || n += 1;
inc();
// println!("{n}"); // ❌ 有时会：inc 还活着，可变借用未结束
drop(inc);          // 先结束闭包
println!("{n}");    // ✅
```

**3. 该 `move` 没 `move`**

把闭包扔进线程 / `spawn` 却还在借局部变量 → 生命周期报错。  
办法：`move`，或让借用的数据活得足够久（`'static` / 拥有的 `Arc` 等，后文展开）。

**4. 和 `fn` 指针搞混**

只接受 `fn(i32) -> i32`（函数指针）的 API， **不能** 传入带捕获的闭包。  
能接受闭包的 API 一般写 `impl Fn…` 或泛型 `F: Fn…`。

----

# 动手实验

```bash
cd code
cargo run -p lang-functions-closures
```

建议自己改一改：

1. 给 `answer_ok` 最后一行加上分号，读报错；  
2. 对 `String` 做 `move` 闭包后，在外面 `println` 它；  
3. 对 `FnOnce` 示例连续调用两次 `consume()`；  
4. 把 `call_twice(&hello)` 换成传入会修改计数的闭包，看为何需要 `FnMut` 版本。

----

# 三句话带走

1. **函数靠最后一行表达式返回**；签名里的 `&` / `&mut` / `T` 就是所有权契约。  
2. **闭包默认借、`move` 则抢**；活得比当前栈久就必须抢（或共享所有权）。  
3. **`Fn` / `FnMut` / `FnOnce`** 描述「能调几次、会不会改环境」——写回调时按需选型。

下一章：[《生命周期》](lifetimes.md)。  
若还没啃透移动与借用，先回 [《所有权与借用》](ownership.md)。

----

# 附：本章生词表

- **`fn`**：函数定义关键字。  
- **表达式返回**：函数/块最后一行无分号表达式作为值返回。  
- **闭包（closure）**：可捕获环境的匿名函数。  
- **捕获（capture）**：闭包使用外层局部变量的方式（借 / 可变借 / move）。  
- **`move` 闭包**：强制按所有权（或 Copy）捕获，而不是借用。  
- **`Fn`**：可多次调用、仅不可变使用捕获的闭包特质。  
- **`FnMut`**：可多次调用、可可变使用捕获的闭包特质。  
- **`FnOnce`**：只能调用一次的闭包特质（常因消费了捕获值）。  
- **函数指针 `fn(...)`**：仅指向 `fn` 项、无捕获的瘦指针类型。  
- **高阶函数**：接受或返回函数/闭包的函数。  
- **`async move`**：异步块加上 move 捕获，供 `spawn` 等把任务送走。
