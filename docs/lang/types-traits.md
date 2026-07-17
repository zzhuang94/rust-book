# 这一章要解决什么

> 代码：[`code/lang-types-traits/`](../../code/lang-types-traits/)　  
> 运行：在 `code/` 目录下执行 `cargo run -p lang-types-traits`

> 前置是 [《所有权与借用》](ownership.md)。 [《Rust 语法底座》](../start/syntax-primer.md) 给过 `Option`/`Result`/`trait` 的生存速览；  
> 这一篇系统讲：`enum` 为什么强大、`match` 怎么用、泛型怎么做到零成本、`trait` 的静态/动态分发——最后这个是 Go interface 用户最该读的一节。

`match` 和泛型这里给的是「在类型系统里够用」的一层；想深挖，它们各有专章：[《模式匹配与枚举》](pattern-matching.md)、  
[《泛型与 trait bound》](generics.md)。

----

# struct 与方法

> Go 里数据是 `struct`、方法用 `func (r Recv)` 挂上去。Rust 几乎一样：数据写在 `struct` 里，  
> 方法写在 `impl` 块里，只是「方法怎么对待你的数据」写得更明确。

```rust
struct User {
    name: String,
    age: u8,
}

impl User {                                    // 方法都写在 impl 块里
    fn new(name: impl Into<String>) -> Self {  // 关联函数（无 self）≈ 构造函数
        User { name: name.into(), age: 0 }
    }
    fn greet(&self) -> String {                // &self：借用自己（只读方法）
        format!("我是 {}", self.name)
    }
    fn birthday(&mut self) {                    // &mut self：可变借用（会修改自己）
        self.age += 1;
    }
    fn into_name(self) -> String {             // self：拿走所有权（消费掉自己）
        self.name
    }
}
```

几个随手记的点：

- `User::new` 这种 **没有 `self`** 的叫「关联函数」，约等于构造函数，用 `User::new(...)` 调用；  
  `Self`（大写）是「当前这个类型」的别名。
- 字面量构造 `User { name, age: 0 }`：字段名和变量名同名时可简写。
- 更新语法 `User { age: 30, ..old }`：其余字段从 `old` 搬来——注意是 **move**，  
  `old` 里非 Copy 的字段随之作废。
- `#[derive(Debug, Clone, Default)]` 一行白得一堆常用能力（下面「转换 trait 全家」会讲）。

----

# 三种 self 接收者

> `impl` 里方法的第一个参数——`&self`、`&mut self`、`self`——就是 [《所有权与借用》](ownership.md) 借用规则在方法上的投影。  
> 方法签名本身就在告诉调用者「我会怎么对待你」。

| 接收者 | 含义 | Go 对照 |
| --- | --- | --- |
| `&self` | 只读看看，不改 | `func (u User)` 值接收者（近似） |
| `&mut self` | 要修改（调用方那边得是 `mut` 的） | `func (u *User)` 指针接收者（近似） |
| `self` | 消费掉，调用后原值作废 | 无对应——Go 表达不了「用完即毁」 |

最后一行是 Go 没有的：`self`（不带 `&`）方法会 **拿走所有权**，调用完这个对象就没了。典型用途是「转换类」方法，  
比如 `into_name(self) -> String` 把 `User` 拆开、只留下 `name`。

----

# enum 能带数据

> 这是 Rust 类型系统最强的一块，也是和 Go 差别最大的地方。Go 的枚举是 `iota` 常量——只是名字好看的整数。  
> Rust 的 `enum` 里， **每个变体可以携带不同形状的数据**。

```rust
enum Shape {
    Circle { r: f64 },              // 结构体式变体
    Rect(f64, f64),                 // 元组式变体
    Point,                          // 空变体（≈ Go 的 iota 那种）
}

enum Command {
    Get(String),
    Set { key: String, value: String, ttl: Option<u64> },
    Quit,
}
```

它的杀手级用途是 **给「多选一」的业务状态建模** ——支付方式、消息类型、状态机的状态。Go 里同样的需求，要么「一个大 struct + 只用其中几个字段 + 靠注释约定」，  
要么「interface + 类型断言」——两者都没法让编译器帮你把守「有没有漏处理某种情况」。

而 Rust 的 `enum` 配合 `match` 的 **穷尽性检查**：新加一个变体，所有漏处理它的地方 **全部编译报错**，  
带你走遍每一个需要改的点。 [《接入 Redis》](../http/redis.md) 里「新增一种错误类型、编译器提醒你每处都要处理」就是这个机制。

其实你早就在用 `enum` 了：`Option<T>` 和 `Result<T, E>` 就是标准库里最有名的两个 `enum`。

----

# match 不只是 switch

> `match` 长得像 Go 的 `switch`，但强在两点：能 **解构** 出变体里的数据，还会 **强制你覆盖所有情况**。  
> 这里给够用的一层，花式玩法见 [《模式匹配与枚举》](pattern-matching.md)。

```rust
match cmd {
    Command::Quit => println!("再见"),
    Command::Get(key) => println!("读 {key}"),                       // 解构出数据
    Command::Set { key, ttl: Some(t), .. } => println!("写 {key}，ttl={t}"),  // 嵌套匹配 + 忽略其余字段
    Command::Set { key, ttl: None, .. } => println!("永久写 {key}"),
}
```

必须掌握的几招：

- **穷尽性**：漏掉一个变体就是 E0004 编译错误；确实想兜底用 `_ => ...`（但对自己定义的 `enum` 慎用 `_`——它会吃掉「新变体的编译器提醒」）；
- **守卫**：`Some(n) if n > 100 => ...`——在模式之上再加一个条件；
- **绑定 `@`**：`n @ 1..=9 => ...`——既匹配范围、又把值绑到 `n`；
- **或模式**：`1 | 2 | 3 => ...`；
- **`match` 是表达式**：`let x = match ... { ... };`，每个分支的值类型必须一致（[《axum 入门》](../http/axum.md) 里 handler 返回值那个坑的根源）。

还有两个轻量形态（[《Rust 语法底座》](../start/syntax-primer.md) 见过）：`if let` 只关心一个分支；  
`let else` 适合「不匹配就提前返回」：

```rust
let Some(user) = find_user(id) else {
    return Err("没这人".into());     // 不匹配的分支必须"发散"（return / break / panic）
};
// 这里 user 已经拿到，主线代码不缩进 —— 对照 Go 的 if err != nil 早返回风格，很顺手
```

----

# 泛型是类型参数

> 泛型让你「一份代码，适配多种类型」。这里给类型系统层面够用的认知，`where`、关联类型上界、`impl Trait` 的花式用法见 [《泛型与 trait bound》](generics.md)。

```rust
fn largest<T: PartialOrd>(list: &[T]) -> &T {
    let mut max = &list[0];
    for x in list {
        if x > max { max = x; }
    }
    max
}

struct Pair<T> { a: T, b: T }

impl<T: std::fmt::Display> Pair<T> {       // 只给"满足约束的 T"实现方法
    fn show(&self) { println!("{} {}", self.a, self.b); }
}
```

三点要点：

- `<T: PartialOrd>` 里的 `PartialOrd` 是 **约束**（trait bound）：「`T` 必须能比大小」。  
  没有约束的 `T` 什么都干不了（和 Go 泛型的 constraints 思路一致）；
- 约束一多就用 `where` 子句排版：`fn f<T>(x: T) where T: Clone + Send + 'static { ... }`；
- **单态化**：编译器为每个实际用到的 `T` 生成一份专属代码（`largest::<i32>` 和 `largest::<String>` 是两个真实存在的函数），  
  运行期 **零开销**。Go 泛型是「字典传参 + 部分单态化」的混合，通常留有一点运行时成本。这就是「零成本抽象」的典型样本。

----

# trait 的默认方法

> [《Rust 语法底座》](../start/syntax-primer.md) 讲过 `trait` 的基本形态（显式 `impl`、  
> 方法要 `use` 进来）。这里补第一块进阶：trait 可以自带默认实现。

```rust
trait Greeter {
    fn name(&self) -> String;                    // 必须由实现者提供
    fn greet(&self) -> String {                  // 默认实现，实现者可覆盖也可白用
        format!("你好，我是 {}", self.name())
    }
}
```

实现者只要写 `name`，就白得一个 `greet`。这是 Rust 版的「接口自带复用」——Go 的 interface **做不到带实现**，  
你得靠嵌入 struct 绕。默认方法让你把「共同行为」收进 trait，实现者只填「各自不同的那部分」。

----

# 关联类型

> trait 里可以声明一个「输出类型」，由每个实现者去指定。它叫关联类型（associated type）。

```rust
trait Iterator {
    type Item;                          // 关联类型：由实现者指定具体是什么
    fn next(&mut self) -> Option<Self::Item>;
}
```

和「泛型参数 `trait Iterator<T>`」的区别，是一个容易绕的点，掰开说：

- **关联类型** 说的是「每个实现者 **只有一种** `Item`」——`Vec` 的迭代器产出什么，是唯一确定的；
- **泛型参数** 说的是「同一个类型可以有 **多种** 实现」。

[《async 基础》](../async/basics.md) 里 `Future` 的 `Output` 就是关联类型（一个 Future 完成后产出什么，  
是定死的）。

----

# 孤儿规则

> 有一条限制会在你想「给某个类型加个 trait 实现」时挡住你：`impl 某trait for 某类型` 要求 **trait 和类型里，  
> 至少有一个是你这个 crate 自己定义的**。

这条规则叫孤儿规则（orphan rule）。它的目的是防止两个依赖各自「给别人的类型实现别人的 trait」，  
最后打架。比如你不能在自己的 crate 里给标准库的 `Vec` 实现别人库的 `Serialize`。

绕过的标准手法是 **newtype**：用一个单字段元组结构体包一层，这个包装类型是你自己的，随便 `impl`：

```rust
struct MyVec(Vec<u8>);          // newtype：包一层，就成了"我的类型"
// 现在可以随意 impl 任何 trait for MyVec
```

[《接入 Redis》](../http/redis.md) 里「为什么要自己定义 `AppError`、而不是直接给别人的 `RedisError` 实现 `IntoResponse`」，  
根子就是孤儿规则。

----

# 静态分发与动态分发

> **这一节 Go interface 用户务必读。** Rust 里「面向接口编程」有两种写法，而 Go 只有其中一种（动态那种）。

```rust
// 写法一：impl Trait / 泛型 —— 静态分发
fn print_static(g: impl Greeter) { println!("{}", g.greet()); }
//  编译期为每种实际类型生成专属版本（单态化），调用是直接调用，可内联，零开销

// 写法二：dyn Trait —— 动态分发
fn print_dyn(g: &dyn Greeter) { println!("{}", g.greet()); }
//  运行期通过 vtable（虚函数表）查找方法；胖指针 =（数据指针, vtable 指针）
```

两者的取舍：

| | `impl Trait`（静态） | `dyn Trait`（动态） | Go interface |
| --- | --- | --- | --- |
| 方法调用 | 直接调用，可内联 | 查 vtable，间接调用 | 查 itab，间接调用 |
| 代价 | 编译慢些、二进制大些 | 每次调用一次间接跳转 | 同左（且常伴堆分配） |
| 能否放进同一个 Vec | ❌ 各是各的类型 | ✅ `Vec<Box<dyn Greeter>>` | ✅ `[]Greeter` |
| 何时用 | 默认首选 | 需要「运行时才知道是谁」的异构集合/插件 | 唯一选择 |

**给 Go 程序员的翻译**：Go 的 interface 永远走 vtable（动态分发）——你习以为常的那点开销，  
在 Rust 里是 **可选项**。经验法则：**默认写泛型 / `impl Trait`；只有真的需要「一个列表装各种实现」时，  
才 `Box<dyn Trait>`**。axum 的 handler、serde 的序列化全走静态分发——这是 Rust Web 框架比同类快的一个真实原因。

----

# 转换 trait 全家

> 想写出「好用的 API」，一大半功夫在这一组转换 trait 上。它们让你的函数「参数宽容、内部收拢」。

| trait | 作用 | 典型使用处 |
| --- | --- | --- |
| `From<T>` / `Into<T>` | 类型转换（`impl From` 会自动白得 `Into`） | `?` 的错误转换、`"x".into()` |
| `TryFrom` / `TryInto` | 可能失败的转换，返回 `Result` | 数值窄化、带校验的构造 |
| `AsRef<str>` / `AsRef<Path>` | 「能便宜地当 `&str`/`&Path` 看」 | 函数参数收 `String`/`&str` 通吃 |
| `Display` | 面向用户的文本（`{}`） | 实现它就自动获得 `.to_string()` |
| `Debug` | 面向开发者的文本（`{:?}`，一般 `derive`） | 日志、断言 |
| `Default` | 默认值 | `..Default::default()` 填充剩余字段 |
| `PartialEq`/`Eq`/`Hash`/`Ord` | 比较/哈希（一般 `derive`） | 当 `HashMap` 的 key、排序 |

一个天天用的惯用法——参数用 `impl Into<String>`，让调用方传 `&str` 或 `String` 都行：

```rust
fn set_name(&mut self, name: impl Into<String>) {   // 调用方传 &str 或 String 都可以
    self.name = name.into();                         // 内部统一收成 String
}
```

----

# 没有继承怎么办

> Rust 没有类继承。但你作为 Go 程序员本来就习惯用组合，迁移成本不大——只要记住一个差异，再避开一个坑。

那个差异是：**Rust 没有 Go 的「嵌入自动提升方法」**。Go 里把 `Base` 嵌进 `Derived`，  
`Base` 的方法就自动成了 `Derived` 的方法；Rust 不会自动做这件事。复用手段按优先级：

1. **组合 + 手写转发**：字段里放部件，方法里调它（转发就转发，别嫌啰嗦）；
2. **trait 默认方法**：把共同行为放进 trait 的默认实现；
3. **泛型 + 约束**：算法写一份，用约束描述「部件能干什么」；
4. ⚠️ **反模式警告**：**别用 `Deref` 模拟继承**（让 `Deref<Target=Base>` 假装子类）——这是社区公认的坑，  
   方法解析会出现诡异行为。

----

# 动手实验

> 挑两三个亲手做，比读十遍都管用。

1. **感受穷尽性**：定义一个 3 变体的 `Event` enum + 一个 `match` 处理它；再加第 4 个变体，  
   看所有 `match` 处的编译报错把你带到每个漏网点；
2. **三种 self**：给 `User` 写 `&self`/`&mut self`/`self` 三个方法，分别在非 `mut` 变量、  
   `mut` 变量上调用，观察哪些编译不过、`self` 方法调用后原变量是否作废（复习 [《所有权与借用》](ownership.md) 的 move）；
3. **静态 vs 动态**：写两个类型实现同一个 trait，分别用 `impl Trait` 参数和 `Vec<Box<dyn Trait>>` 去消费它们；  
   再试着把两种类型塞进 `Vec<impl Trait>`，看编译器怎么拒绝你；
4. **From 链**：给一个自定义错误实现 `From<std::io::Error>`，然后在函数里用 `?` 直接传播 io 错误——亲手复刻 [《接入 Redis》](../http/redis.md) 里 `AppError` 的机制。

----

# 三句话带走

> 只记三句的话，就这三句。

1. **enum + match 是 Rust 建模的心脏**：变体能携带数据、匹配有穷尽检查——「新增一种情况，编译器带你走遍所有需要改的地方」。
2. **泛型是单态化的零成本抽象；trait 的分发方式可选**：默认静态（`impl Trait`），需要异构集合才动态（`dyn`）——Go interface 只有动态这一档。
3. **好用的 API 靠转换 trait 全家**（`From`/`Into`/`AsRef`/`Display`/`Default`），  
   复用靠组合与默认方法——没有继承，也不需要。

----

# 附：本章生词表

- **关联函数 / `Self`** ——`impl` 块里无 `self` 的函数（如 `User::new`）；`Self` 是「当前类型」的别名。
- **结构体更新语法 `..old`** ——其余字段从 `old` move 过来；`old` 里非 Copy 字段随之作废。
- **守卫 / `@` 绑定 / 或模式** ——`match` 的进阶三件套：`Some(n) if n>0`、`n @ 1..=9`、`A | B`。
- **`let else`** ——「不匹配就发散（return/break）」的模式绑定，Go 早返回风格的 Rust 对应。
- **单态化（monomorphization）** ——编译器按实际类型展开泛型，运行期零开销；代价是编译时间和二进制体积。
- **trait bound / `where`** ——泛型约束；约束一多就换 `where` 子句排版。
- **关联类型 `type Item`** ——「每个实现者唯一确定」的输出类型；如 `Future::Output`、`Iterator::Item`。
- **孤儿规则（orphan rule）** ——`impl` 的 trait 和类型至少一个是本地的；绕法是 newtype。
- **`dyn Trait` / vtable / 胖指针** ——动态分发三件套；`Box<dyn T>` 用来装异构集合。
- **newtype** ——`struct Wrapper(Inner);` 单字段元组结构体：绕孤儿规则，或给同一类型不同语义（如 `UserId(u64)` 防止和 `OrderId(u64)` 混用）。
