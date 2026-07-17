# 泛型解决什么

> 代码：[`code/lang-generics/`](../../code/lang-generics/)　  
> 运行：在 `code/` 目录下执行 `cargo run -p lang-generics`

> Go 1.18 才加泛型，且刻意做得克制；Rust 的泛型是语言核心，标准库和框架代码里到处都是。看懂它，你才读得顺 axum、  
> serde、sqlx 那些库的签名。

泛型要解决的问题一句话：**同一套逻辑，别为每种类型重写一遍**。写一个「找最大值」的函数，不该 `largest_i32`、  
`largest_str` 各写一份。 [《类型系统与 trait》](types-traits.md) 给过泛型的入门；  
这一篇把 `trait bound`、`where`、`impl Trait`、关联类型、`dyn` 之间的取舍讲透。

----

# 泛型函数与结构体

> 泛型参数写在尖括号 `<T>` 里，函数、结构体、枚举、方法都能用。

```rust
// 泛型函数：T 是"类型参数"，调用时由实际类型填进去
fn first<T>(list: &[T]) -> &T {
    &list[0]
}

// 泛型结构体：一个 Pair 能装任意类型的两个值
struct Pair<T> {
    a: T,
    b: T,
}

// 给泛型结构体写方法：impl 后也要带 <T>
impl<T> Pair<T> {
    fn new(a: T, b: T) -> Self {
        Pair { a, b }
    }
}

// 泛型枚举你其实天天用：Option<T>、Result<T, E> 就是
enum MyOption<T> {
    Some(T),
    None,
}
```

`impl<T> Pair<T>` 里那个 `<T>` 别漏：第一个 `<T>` 是「声明一个类型参数」，后面的 `Pair<T>` 是「给这个泛型类型实现方法」。

----

# trait bound 是约束

> 光有 `T` 还不够——编译器不知道 `T` 能干什么，于是 `T` 上什么方法都调不了。你得用 **trait bound** 告诉它「`T` 至少实现了哪些 trait」。

```rust
// ❌ 编不过：编译器不知道 T 能不能用 > 比较
// fn largest<T>(list: &[T]) -> &T {
//     let mut max = &list[0];
//     for x in list { if x > max { max = x; } }   // error: T 可能不能比大小
//     max
// }

// ✅ 加约束：T 必须实现 PartialOrd（能比大小）
fn largest<T: PartialOrd>(list: &[T]) -> &T {
    let mut max = &list[0];
    for x in list {
        if x > max { max = x; }
    }
    max
}
```

`T: PartialOrd` 读作「`T` 必须实现 `PartialOrd`」。这和 Go 泛型的 constraints 思路完全一致——Go 里 `[T constraints.Ordered]`，  
Rust 里 `<T: PartialOrd>`。

多个约束用 `+` 连起来：

```rust
fn show_all<T: std::fmt::Display + Clone>(items: &[T]) {
    for x in items {
        let copy = x.clone();          // 因为约束了 Clone，才能 .clone()
        println!("{copy}");            // 因为约束了 Display，才能 {} 打印
    }
}
```

----

# where 让签名清爽

> 约束一多，挤在尖括号里就很难读。`where` 子句把约束挪到签名后面，专门排版。

```rust
// 挤在一起（约束多了很难读）：
fn process<T: Clone + std::fmt::Debug, U: Default + PartialEq>(t: T, u: U) { /* ... */ }

// 用 where 摊开（等价，但清爽）：
fn process<T, U>(t: T, u: U)
where
    T: Clone + std::fmt::Debug,
    U: Default + PartialEq,
{
    /* ... */
}
```

两种写法完全等价。约束简单时用前者，复杂时用 `where`——你在标准库和框架源码里会见到大量 `where`。

----

# 单态化是零成本

> Rust 泛型「零成本」的秘密叫单态化：编译器在编译期，为你 **实际用到的每一种类型** 各生成一份专属代码。

```rust
let a = largest(&[3, 7, 2]);          // 用到 i32
let b = largest(&["x", "yy", "z"]);   // 用到 &str
```

编译后，实际上存在 `largest::<i32>` 和 `largest::<&str>` 两个 **真实的、各自独立** 的函数——就好像你手写了两份。  
所以泛型调用是「直接调用」，可以内联，运行期 **没有任何额外开销**（不像动态分发要查表）。

代价在别处：**编译时间更长、二进制体积更大**（每种类型一份代码）。对照 Go：Go 泛型用的是「字典传参 + 部分单态化」的折中，  
通常留有一点运行时成本，换取更小的二进制。两种取舍没有绝对好坏，但 Rust 选了「运行期零开销」这一头。

----

# impl Trait 两种用法

> `impl Trait` 是泛型的一种简写，出现在两个位置、含义不同——这是个常见困惑点，拆开说。

**用在参数位置** = 泛型参数的简写：

```rust
// 下面两行完全等价：
fn greet(g: impl std::fmt::Display) { println!("{g}"); }
fn greet<T: std::fmt::Display>(g: T) { println!("{g}"); }
```

「收一个实现了 `Display` 的东西」，`impl Display` 比写 `<T: Display>` 更短。

**用在返回位置** = 「我返回某个实现了该 trait 的类型，但具体类型不告诉你」：

```rust
// 返回一个迭代器，但不想（也很难）写出它那一长串具体类型
fn evens(max: u32) -> impl Iterator<Item = u32> {
    (0..max).filter(|n| n % 2 == 0)
}
```

返回位置的 `impl Trait` 特别适合闭包和迭代器——它们的真实类型是编译器生成的、写都写不出来的一长串，  
用 `impl Iterator<...>` 一笔带过。异步函数 `async fn f() -> T` 的真实返回类型其实就是 `impl Future<Output = T>`。

----

# 关联类型 vs 泛型参数

> trait 里「输出类型」既能用泛型参数 `<T>`，也能用关联类型 `type Item`。什么时候用哪个？这是个高频困惑，一句话讲清。

```rust
// 关联类型：每个实现者只有"一种"输出
trait Producer {
    type Output;                       // 关联类型
    fn produce(&self) -> Self::Output;
}

// 泛型参数：同一个类型可以有"多种"实现
trait Convert<T> {
    fn convert(&self) -> T;
}
```

判断口诀：

- **「一个实现者只该有一种」→ 用关联类型**。比如 `Iterator` 的 `Item`：一个具体迭代器产出什么，  
  是唯一确定的，不该让你 `impl Iterator<A>` 又 `impl Iterator<B>`。
- **「同一类型想对多种目标都实现」→ 用泛型参数**。比如「能转成好几种类型」，就 `impl Convert<i32> for X` 再 `impl Convert<String> for X`。

标准库里 `Iterator::Item`、`Future::Output`、`Deref::Target` 都是关联类型；  
`From<T>`、`Into<T>` 用泛型参数（因为一个类型确实能从/到好多种类型转换）。

----

# 泛型带生命周期

> 生命周期参数 `'a` 其实也是一种泛型参数——只不过它约束的是「引用能活多久」而不是「类型是什么」。两者常常一起出现。

```rust
// <'a, T>：既有生命周期参数 'a，又有类型参数 T
// 意思是：Wrapper 借用了一个 T，且它活不过被借的那个 T
struct Wrapper<'a, T> {
    inner: &'a T,
}

impl<'a, T: std::fmt::Display> Wrapper<'a, T> {
    fn show(&self) {
        println!("{}", self.inner);
    }
}
```

看到 `<'a, T>` 别慌：`'a`（带撇号的）是生命周期参数，`T` 是类型参数，它们在尖括号里并列声明。  
生命周期的完整讲解见 [《生命周期》](lifetimes.md)；所有权与借用见 [《所有权与借用》](ownership.md)。

----

# 一次实现给一片

> 你可以「一次性给所有满足某约束的类型」实现一个 trait，这叫 blanket impl（覆盖实现）。标准库大量用它，  
> 理解它才看得懂那些「凭空多出来的方法」。

```rust
trait Summary {
    fn summarize(&self) -> String;
}

// 给"所有实现了 Display 的类型"一次性实现 Summary
impl<T: std::fmt::Display> Summary for T {
    fn summarize(&self) -> String {
        format!("摘要：{self}")
    }
}
```

写完这一段， **任何** 实现了 `Display` 的类型（`i32`、`String`、你自己的类型……）就自动都有了 `.summarize()`。  
标准库里 `impl<T: Display> ToString for T` 就是这么干的——这就是为什么随便一个能 `{}` 打印的类型都自带 `.to_string()`。

----

# 何时用 dyn 而非泛型

> 泛型（静态分发）是默认首选，但有一种情况非 `dyn`（动态分发）不可：**一个集合里要装「多种不同的具体类型」**。

```rust
trait Draw {
    fn draw(&self);
}

struct Circle;
struct Square;
impl Draw for Circle { fn draw(&self) { println!("○"); } }
impl Draw for Square { fn draw(&self) { println!("□"); } }

fn main() {
    // ❌ Vec<impl Draw> 不行：Vec 要求所有元素是"同一个"具体类型
    // let shapes = vec![Circle, Square];   // 类型不一致

    // ✅ Vec<Box<dyn Draw>>：装"任何实现了 Draw 的东西"，运行期查 vtable
    let shapes: Vec<Box<dyn Draw>> = vec![Box::new(Circle), Box::new(Square)];
    for s in &shapes {
        s.draw();
    }
}
```

取舍回顾（详见 [《类型系统与 trait》](types-traits.md) 的分发一节）：

- **泛型 / `impl Trait`（静态）**：零开销、可内联，但一个泛型实例只能对应一种具体类型—— **默认用它**；
- **`dyn Trait`（动态）**：每次调用查一次 vtable，有一点点开销，但能把不同类型装进同一个集合—— **需要异构集合/插件时才用**。

给 Go 程序员：你习惯的 `[]SomeInterface` 就是 Rust 的 `Vec<Box<dyn SomeTrait>>`；  
只是在 Rust 里，它是「需要时才付出的成本」，而非默认。

----

# 三句话带走

1. **泛型 = 类型参数 + trait bound**：`<T>` 提供占位，`T: Trait` 告诉编译器 `T` 能干什么；  
   约束多了用 `where`。
2. **单态化让泛型零运行时开销**：编译期按实际类型各生成一份代码，代价是编译更慢、二进制更大。
3. **静态优先、异构才 dyn**：默认泛型/`impl Trait`；只有「一个集合装多种类型」时才 `Box<dyn Trait>`。  
   关联类型用于「一个实现者只有一种输出」，泛型参数用于「一个类型有多种实现」。

----

# 附：本章生词表

- **类型参数 `<T>`** ——泛型的占位符，调用/使用时由实际类型填入。
- **trait bound（约束）** ——`T: Trait`，规定 `T` 至少实现了哪些 trait，否则 `T` 上什么都调不了。
- **`where` 子句** ——把复杂约束从尖括号挪到签名后面，专门排版；与内联写法等价。
- **单态化（monomorphization）** ——编译期为每种实际类型各生成一份专属代码，运行期零开销。
- **`impl Trait`** ——参数位置是「`<T: Trait>` 的简写」；返回位置是「返回某个实现了该 trait 的匿名类型」。
- **关联类型 `type Item`** ——trait 的输出类型，「每个实现者唯一确定」；对比泛型参数「一个类型可多种实现」。
- **生命周期参数 `'a`** ——一种特殊的泛型参数，约束引用能活多久，常与类型参数并列。
- **blanket impl（覆盖实现）** ——`impl<T: Bound> Trait for T`，一次给所有满足约束的类型实现；  
  如 `ToString`。
- **`dyn Trait` / vtable** ——动态分发；`Box<dyn Trait>` 用来把不同具体类型装进同一集合。
