# 为什么模式匹配好用

> 代码：[`code/lang-pattern-matching/`](../../code/lang-pattern-matching/)　  
> 运行：在 `code/` 目录下执行 `cargo run -p lang-pattern-matching`

> Go 里你用 `switch`、`if`、类型断言 `v, ok := x.(T)` 拼出来的一堆分支逻辑，在 Rust 里往往一个 `match` 就写完了，  
> 而且编译器会盯着你「有没有漏掉某种情况」。这一章把模式匹配的全套招式过一遍。

[《类型系统与 trait》](types-traits.md) 里已经给过 `match` 的基本形态。这一篇是「深水区」：  
解构、守卫、范围、`@` 绑定、`if let`/`while let`/`let else`、`matches!`——写业务代码时你会天天用到它们。

先记住一个核心区别：Go 的 `switch` 是「按值/类型挑一个分支执行」；Rust 的 `match` 是「按 **结构** 匹配，  
顺手把里面的数据 **拆出来**，而且必须覆盖所有可能」。「拆出来」和「必须覆盖」这两点，是它比 `switch` 强的地方。

----

# match 的穷尽性

> `match` 最值钱的特性：**你必须处理所有可能的情况，漏一个就编译不过**。这把「忘了处理某个 case」从运行时 bug 变成了编译错误。

```rust
enum Status { Active, Paused, Deleted }

fn describe(s: Status) -> &'static str {
    match s {
        Status::Active => "运行中",
        Status::Paused => "已暂停",
        Status::Deleted => "已删除",
        // 少写任何一个变体，就会 error[E0004]: non-exhaustive patterns
    }
}
```

对照 Go：Go 的 `switch` 少写一个 `case` 编译器不会吭声，运行时悄悄走 `default` 或什么都不做。  
Rust 逼你写全——好处是 **将来给 `Status` 加一个新变体（比如 `Archived`）时，所有没更新的 `match` 全部编译报错**，  
带你走遍每一处需要改的地方。这是大型项目重构时的救命特性。

正因如此， **对自己定义的 enum 要慎用兜底的 `_`** ——`_` 会「吃掉」这个提醒，加了新变体也不报错了：

```rust
match s {
    Status::Active => "运行中",
    _ => "其它",          // ⚠️ 加了新变体也不会提醒你，除非确实想「其余全归一类」
}
```

----

# 解构取出数据

> `match` 能一边匹配、一边把变体/结构体/元组里的数据「拆」到变量里。这是它和 `switch` 最不一样的地方。

```rust
enum Msg {
    Quit,
    Move { x: i32, y: i32 },     // 结构体式变体
    Write(String),               // 元组式变体
    Color(u8, u8, u8),
}

fn handle(m: Msg) {
    match m {
        Msg::Quit => println!("退出"),
        Msg::Move { x, y } => println!("移动到 ({x}, {y})"),  // 拆出 x、y
        Msg::Write(text) => println!("文本：{text}"),          // 拆出 text
        Msg::Color(r, g, b) => println!("颜色 #{r:02x}{g:02x}{b:02x}"),
    }
}
```

解构能嵌套、能用在元组和结构体上，不限于 enum：

```rust
let point = (0, 7);
match point {
    (0, 0) => println!("原点"),
    (0, y) => println!("在 Y 轴上，y={y}"),   // 第一位必须是 0，第二位绑到 y
    (x, 0) => println!("在 X 轴上，x={x}"),
    (x, y) => println!("其它点 ({x}, {y})"),
}

struct User { name: String, age: u8 }
let u = User { name: "张三".into(), age: 30 };
let User { name, age } = u;                   // 直接在 let 里解构，一次拆出所有字段
println!("{name} 今年 {age}");
```

----

# 守卫加条件

> 光靠「形状」不够时，可以在模式后面加一个 `if` 条件，叫「守卫（guard）」。形状对上、且条件成立，才算匹配。

```rust
let n = 128;
match n {
    x if x < 0 => println!("负数"),
    0 => println!("零"),
    x if x % 2 == 0 => println!("正偶数 {x}"),   // 形状匹配任意值，再用守卫筛
    x => println!("正奇数 {x}"),
}
```

守卫常和 `Option` 配合，表达「有值、且值满足某条件」：

```rust
let score = Some(85);
match score {
    Some(s) if s >= 90 => println!("优秀"),
    Some(s) if s >= 60 => println!("及格：{s}"),
    Some(s) => println!("不及格：{s}"),
    None => println!("缺考"),
}
```

----

# 范围与或模式

> 两个常用简写：`..=` 匹配一个范围，`|` 匹配「多个之一」。

```rust
let c = 'k';
match c {
    'a'..='z' => println!("小写字母"),      // 范围模式：a 到 z（闭区间）
    'A'..='Z' => println!("大写字母"),
    '0'..='9' => println!("数字"),
    _ => println!("其它字符"),
}

let day = 6;
match day {
    1 | 2 | 3 | 4 | 5 => println!("工作日"),   // 或模式：1 到 5 之一
    6 | 7 => println!("周末"),
    _ => println!("非法"),
}
```

----

# @ 绑定值

> 有时你既想「用范围/条件筛」，又想「把匹配到的原始值留下来用」。`@` 就干这个：`名字 @ 模式`——匹配这个模式，同时把值绑到名字上。

```rust
let n = 5;
match n {
    x @ 1..=9 => println!("个位数：{x}"),        // 匹配 1..=9，同时把值绑到 x
    x @ 10..=99 => println!("两位数：{x}"),
    x => println!("其它：{x}"),
}
```

没有 `@` 的话，`1..=9 => ...` 能匹配但拿不到具体值；加了 `x @` 就两全其美。

----

# if let 只看一种

> 很多时候你只关心「是不是某一种情况」，为此写整个 `match` 太重。`if let` 就是「只匹配一个模式」的轻量写法。

```rust
let config: Option<String> = Some("debug".into());

// 啰嗦的 match 写法：
match config {
    Some(mode) => println!("模式：{mode}"),
    None => {}                    // 另一支什么都不做，纯凑数
}
```

用 `if let` 一行搞定，还能带 `else`：

```rust
let config: Option<String> = Some("debug".into());
if let Some(mode) = config {
    println!("模式：{mode}");     // 只有是 Some 才进来，mode 是里面的值
} else {
    println!("用默认模式");
}
```

对照 Go：`if v, ok := m[k]; ok { ... }` 那个 `ok` 判断，Rust 里就是 `if let Some(v) = ...`。

----

# while let 循环取

> `while let` 是「只要还能匹配上，就一直循环」。最典型的用途是「不断从通道/迭代器里取，取到空就停」。

```rust
let mut stack = vec![1, 2, 3];

// pop() 返回 Option：还有元素就是 Some(x)，空了就是 None
while let Some(top) = stack.pop() {
    println!("弹出 {top}");        // 依次打印 3、2、1
}
// 一旦 pop() 返回 None，循环自动结束
```

异步代码里 `while let Some(msg) = rx.recv().await { ... }`（不断收消息直到通道关闭）就是这个模式，  
[《Tokio 运行时》](../async/tokio.md) 会大量用到。

----

# let else 早返回

> `let else` 是「模式匹配版的早返回」，专治 Go 里那种 `if err != nil { return }` 的缩进金字塔。

```rust
fn parse_port(s: &str) -> Result<u16, String> {
    // 匹配成功：port 直接拿到手，主线代码不缩进往下走
    let Ok(port) = s.parse::<u16>() else {
        return Err(format!("不是合法端口：{s}"));   // 匹配失败：必须"发散"（return/break/panic）
    };
    Ok(port)
}
```

它和 `if let` 相反：`if let` 是「匹配上才进块」，`let else` 是「匹配上就继续往下、  
匹配不上就走 else 逃生」。逃生分支必须 `return`/`break`/`panic`（不能往下走），这样编译器才能保证：  
`else` 之后 `port` 一定有值。这正是 Go 的「早返回、主逻辑不缩进」风格的 Rust 对应，非常顺手。

----

# 忽略用 _ 和 ..

> 匹配时不关心的部分，用 `_` 忽略单个值，用 `..` 忽略「其余一片」。

```rust
let tuple = (1, 2, 3, 4, 5);
let (first, .., last) = tuple;       // 只要头和尾，中间用 .. 一笔带过
println!("{first} ... {last}");

struct Config { host: String, port: u16, debug: bool }
let cfg = Config { host: "localhost".into(), port: 8080, debug: true };
let Config { host, .. } = cfg;       // 只取 host，其余字段用 .. 忽略
println!("host={host}");

// 单独一个 _ 忽略一个值，且不绑定（不会触发"未使用变量"警告）
let _ = std::fs::remove_file("tmp"); // 明确表示"我知道有返回值(Result)，故意不管它"
```

`_` 和 `_name` 的区别：`_` 完全不绑定；`_name`（下划线开头的名字）会绑定但不告警——用于「暂时不用、但想留个名字」的场合。

----

# matches! 快速判断

> 只想问「它是不是某个模式」、要一个 `bool` 时，`matches!` 宏最省事。

```rust
let x = Some(5);

// 想要一个 bool：它是不是 Some 且大于 3？
let big = matches!(x, Some(n) if n > 3);      // true
println!("{big}");

// 等价于啰嗦版：
let big = match x {
    Some(n) if n > 3 => true,
    _ => false,
};
```

`matches!` 在写条件、断言、过滤时特别顺手，比如 `list.iter().filter(|s| matches!(s, Status::Active))`。

----

# 对照 Go 的 type switch

> Go 处理「一个接口值到底是什么类型」用 type switch。Rust 里对应的是「对 enum 做 match」——但更安全，  
> 因为类型都是封闭、已知的。

Go 的写法：

```go
switch v := x.(type) {
case int:    fmt.Println("整数", v)
case string: fmt.Println("字符串", v)
default:     fmt.Println("其它")
}
```

Rust 里，你通常先用一个 enum 把「可能的几种」封起来，再 match——编译器能检查穷尽性，而 Go 的 type switch 做不到（接口能装的类型是开放的，  
编译器没法帮你查全）：

```rust
enum Value { Int(i64), Text(String), Flag(bool) }

fn show(v: Value) {
    match v {
        Value::Int(n) => println!("整数 {n}"),
        Value::Text(s) => println!("字符串 {s}"),
        Value::Flag(b) => println!("布尔 {b}"),
    }
}
```

一句话：**Go 用「开放的接口 + type switch」，Rust 用「封闭的 enum + match」** ——后者把「有没有漏处理」的检查交给了编译器。

----

# 三句话带走

1. **`match` 会解构、且强制穷尽**：一边拆出数据，一边逼你覆盖所有情况；对自己的 enum 慎用 `_` 兜底，好留住「新变体提醒」。
2. **一堆轻量形态各司其职**：只看一种用 `if let`、循环取用 `while let`、早返回用 `let else`、  
   要 bool 用 `matches!`。
3. **守卫 / 范围 / 或 / `@` / `..`** 是解构的调味料：`if 条件`、`a..=z`、`A | B`、  
   `x @ 模式`、`..` 忽略其余。

----

# 附：本章生词表

- **模式（pattern）** ——描述「数据长什么样」的模板；匹配成功时可顺带把里面的值绑到变量。
- **穷尽性（exhaustiveness）** ——`match` 必须覆盖所有可能，漏则 E0004；给 enum 加新变体时它带你走遍每处。
- **解构（destructuring）** ——在模式里把元组/结构体/枚举变体里的字段拆到变量。
- **守卫（guard）** ——模式后的 `if 条件`，形状匹配且条件成立才算命中。
- **范围模式 `a..=z`** ——匹配一段连续的值（闭区间）。
- **或模式 `A | B`** ——匹配「其中之一」。
- **`@` 绑定** ——`名字 @ 模式`：匹配该模式的同时，把原值绑到名字。
- **`if let` / `while let` / `let else`** ——只关心一个分支的轻量匹配：进块 / 循环取 / 匹配不上就发散。
- **`_` / `..`** ——`_` 忽略一个值（不绑定）；`..` 忽略「其余一片」字段或元素。
- **`matches!`** ——把「是否匹配某模式」变成一个 `bool` 的宏。
