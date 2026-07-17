# 流程怎么拐弯

> 代码：[`code/lang-control-flow/`](../../code/lang-control-flow/)　  
> 运行：在 `code/` 目录下执行 `cargo run -p lang-control-flow`

> 前置：[《基础类型》](basics.md)。  
> `if` / `for` / `break` 你在 Go 里用到麻木了；Rust 多了几手——  
> **`if` 是表达式**、`loop` 的 `break` 能 **带走一个值**、`for` 遍历默认可能 **move**。  
> 这一章把拐弯抹角的写法对齐到肌肉记忆。

----

# if 是表达式

> Go 的 `if` 是语句，不产生值。  
> Rust 的 `if` 是表达式：两个分支类型必须一致，整个 `if` 可以赋给变量。

```rust
let n = 7;
let parity = if n % 2 == 0 { "偶" } else { "奇" };
//           ↑ 整个 if 的值是 &str，赋给 parity
```

这就是 Rust 版「三元运算」。Go 没有 `?:`，你们习惯写：

```go
var parity string
if n%2 == 0 {
    parity = "偶"
} else {
    parity = "奇"
}
```

Rust 一行搞定，还强迫你：`else` 两边类型对齐——一边返回 `&str`、一边返回 `i32` 会直接编译不过。

条件 **必须是 `bool`**：

```rust
// if n { ... }        // ❌ 期望 bool，找到 i32
if n != 0 { /* ... */ } // ✅ 自己写出判断
```

----

# else if 链

> 读起来和 Go 一模一样；记住它仍然是表达式，常用来算出一个「等级 / 状态」。

```rust
let grade = if score >= 90 {
    "A"
} else if score >= 80 {
    "B"
} else if score >= 60 {
    "C"
} else {
    "D"
};
```

分支很多、还要按结构拆数据时，更地道的是 `match`——见 [《模式匹配与枚举》](pattern-matching.md)。  
`if/else if` 适合「几个布尔条件比大小」；`match` 适合「穷尽一种类型的所有形状」。

----

# loop 与 break 带值

> `loop` = 无限循环，直到 `break`。  
> 大杀器：`break 表达式` 可以把值 **送出** 整个 `loop`，当作表达式结果。

```rust
let mut i = 0;
let found = loop {
    i += 1;
    if i == 3 {
        break i * 10;   // found == 30
    }
};
```

对照 Go：`break` 不能带值，你得在外面准备一个变量再赋：

```go
found := 0
for {
    // ...
    found = i * 10
    break
}
```

Rust 少一个可变外变量，意图更干净：「这个循环在算一个结果」。

`continue` 与 Go 同义：跳过本轮剩余部分，进入下一轮。

----

# while 条件循环

> `while 条件 { ... }`：条件为真就继续。和 Go 的 `for 条件 { }` 同一角色。

```rust
let mut n = 3;
while n > 0 {
    println!("{n}");
    n -= 1;
}
```

习惯差异：Rust 里「故意死循环」更常写 `loop { ... }`，而不是 `while true`——  
语义更直白，也方便挂 `break` 带值。

----

# for 走区间

> Rust **没有** C 风格 `for (i := 0; i < n; i++)`。  
> 数数用区间：`0..n`（半开，不含 n）、`0..=n`（闭合，含 n）。

```rust
for i in 0..3 {          // 0, 1, 2
    println!("{i}");
}
for i in 0..=3 {         // 0, 1, 2, 3
    println!("{i}");
}
for i in (0..6).step_by(2) {  // 0, 2, 4
    println!("{i}");
}
```

| 写法 | 含什么 | Go |
| --- | --- | --- |
| `0..3` | 0,1,2 | `for i := 0; i < 3; i++` |
| `0..=3` | 0,1,2,3 | `for i := 0; i <= 3; i++` |
| `(a..b).step_by(k)` | 步长 k | `for i := a; i < b; i += k` |
| `(0..10).rev()` | 倒序 | 自己改循环变量 |

----

# for 扫集合

> `for x in 集合` 底层是迭代器。  
> **默认会转移所有权**（move）——循环结束后集合往往不能再用。  
> 只看不抢，请写 `for x in &集合`。

```rust
let v = vec!["甲", "乙", "丙"];

for item in &v {           // 借用，v 循环后还在
    println!("{item}");
}
println!("len={}", v.len()); // ✅

// for item in v { ... }   // 这会吃掉 v，后面就不能再用 v 了
```

要下标（对照 Go `for i, x := range v`）：

```rust
for (i, item) in v.iter().enumerate() {
    println!("{i}: {item}");
}
```

更深的三种入口（`iter` / `iter_mut` / `into_iter`）见 [《迭代器》](iterators.md)；  
所有权细节见 [《所有权与借用》](ownership.md)。本章先记住口诀：**只看加 `&`**。

----

# 循环标签

> 多层循环时，`break` / `continue` 默认只作用于 **最内层**。  
> 加标签可以指定跳到哪一层——Go 也有同款 `Outer:` 标签。

```rust
'outer: for x in 1..=3 {
    for y in 1..=3 {
        if x == 2 && y == 2 {
            break 'outer;   // 连外层一起停
        }
    }
}
```

标签名是 `'名字`（前面有单引号），和生命周期的 `'a` 长得像，但这里是 **循环标签**，别混。

----

# 和 Go 的差异速查

| 你想做的事 | Go | Rust |
| --- | --- | --- |
| 三元取值 | 没有，用 if 赋变量 | `let x = if c { a } else { b };` |
| 死循环 | `for { }` | `loop { }` |
| break 带结果 | 不行，用外变量 | `break value` |
| 经典三段 for | `for i:=0; i<n; i++` | `for i in 0..n` |
| range 遍历 | `for i, v := range` | `for (i, v) in xs.iter().enumerate()` |
| 条件必须是 bool | 是 | 是（更严，没弱类型真值） |

----

# 动手实验

```bash
cd code
cargo run -p lang-control-flow
```

建议自己改一改：

1. 给 `if` 两个分支返回不同类型，读报错；  
2. 把 `for item in &v` 改成 `for item in v`，再访问 `v.len()`，看 move 报错；  
3. 去掉 `break 'outer` 的标签，改成裸 `break`，观察 `hits` 变大；  
4. 用 `loop` + `break 值` 写一个「找到第一个大于 10 的平方数」小练习。

----

# 三句话带走

1. **`if` / `loop` 都是表达式**，分支类型要齐，`break` 能送出结果。  
2. **没有 C 式 for**，数数用 `0..n` / `0..=n`。  
3. **`for x in 集合` 默认 move**，只看请 `&集合`。

下一章先去啃承重墙：[《所有权与借用》](ownership.md)；  
再回来读 [《函数与闭包》](functions-closures.md)（闭包捕获依赖所有权心智）。

----

# 附：本章生词表

- **表达式 / 语句**：表达式产生值；语句执行动作。Rust 里 `if`/`loop`/块可以是表达式。  
- **`loop`**：无条件无限循环，靠 `break`/`return` 离开。  
- **`break` 带值**：`break expr` 使整个 `loop` 表达式等于 `expr`。  
- **半开区间 `a..b`**：含 `a` 不含 `b`。  
- **闭合区间 `a..=b`**：两端都含。  
- **`step_by`**：按步长跳着迭代。  
- **`enumerate`**：把迭代器变成 `(下标, 元素)`。  
- **循环标签 `'name`**：给循环命名，供外层 `break`/`continue` 指定目标。  
- **move（预告）**：转移所有权；`for x in v` 常会吃掉 `v`，详见所有权章。
