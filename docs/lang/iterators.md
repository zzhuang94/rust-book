# 用链代替循环

> 代码：[`code/lang-iterators/`](../../code/lang-iterators/)　  
> 运行：在 `code/` 目录下执行 `cargo run -p lang-iterators`

> 前置：[《所有权与借用》](ownership.md)、 [《集合：Vec 与 HashMap》](collections.md)、  
> [《函数与闭包》](functions-closures.md)（链上全是闭包）。  
> 这是 Rust 和 Go 日常写码 **最大的习惯差异**：  
> Go 手写 `for`；Rust 搭一条「挑 → 变 → 截 → 收」的惰性流水线。  
> 性能与手写循环相同（零成本抽象），放心用。

----

# 三种入口

> 遍历集合有三种入口，对应所有权的三种姿势。搞混是新手高频坑。

```rust
let v = vec![1, 2, 3];
for x in v.iter() { }        // x: &i32     —— 借来看（≈ for x in &v）
for x in v.iter_mut() { }    // x: &mut i32 —— 借来改（≈ for x in &mut v）
for x in v.into_iter() { }   // x: i32      —— 拿走所有权（≈ for x in v）
```

**高频坑**：`for x in v`（直接写变量名）走 `into_iter`，  
循环后 `v` 作废（E0382）。 **只想看、循环后还要用 → 写 `for x in &v`。**

带下标（对照 Go `for i, x := range`）：

```rust
for (i, item) in v.iter().enumerate() {
    println!("{i}: {item}");
}
```

----

# 惰性流水线

> 迭代器 **惰性**：适配器只搭水管，不流数据。  
> 末端方法（`collect` / `sum` / `for_each`…）一调用才真正跑。  
> 和 [《async 基础》](../async/basics.md) 里 Future「不 await 不执行」同哲学。

```rust
let names: Vec<String> = users
    .iter()
    .filter(|u| u.age >= 18)          // 挑
    .map(|u| u.name.to_uppercase())   // 变
    .take(10)                         // 截
    .collect();                       // ← 末端：执行并收集
```

没有 `collect`/`sum` 这类消费者，前面的 `filter`/`map` **什么都不做**  
（甚至可以定义为永远不跑的死代码，编译器常能优化掉）。

----

# 适配器速查

> 按用途记一张表，写码时随手翻。

| 用途 | 适配器 |
| --- | --- |
| 变换 | `map`、`filter`、`filter_map`、`flat_map`、`flatten` |
| 截取 | `take`、`skip`、`take_while`、`skip_while`、`step_by` |
| 组合 | `enumerate`、`zip`、`chain`、`rev` |
| 末端·聚合 | `sum`、`count`、`max`/`min`、`fold` |
| 末端·查找 | `find`、`position`、`any`、`all` |
| 末端·收集 | `collect`、`for_each`、`partition` |

```rust
nums.iter().copied().filter(|n| n % 2 == 0);  // 偶数
["a", "b"].iter().zip([1, 2]);                // 并排
nums.iter().any(|&n| n > 5);                  // 是否存在
nums.iter().fold(0, |acc, n| acc + n);        // 万能累积
```

`filter_map`：闭包返回 `Option`，`Some` 留下、`None` 丢掉——过滤+变换一步到位。

对照 Go：这些基本全是手写 for（1.21+ `slices` 包补了一点）。  
心态转变：Go 美学是「循环一眼见底」；Rust 美学是「声明意图，步骤交给链」。

----

# collect 收成什么

> `collect` 是变形金刚：**等号左边要什么容器，它就装成什么**。  
> 又是一次「类型标注驱动行为」。

```rust
let v: Vec<i64> = (1..=5).collect();
let s: String = ['a', 'b'].into_iter().collect();
let m: HashMap<u32, &str> = pairs.into_iter().collect();

// 推不出类型时用 turbofish：
let v = (1..=5).collect::<Vec<_>>();
```

----

# Result 也能 collect

> 最惊艳的一招：`Vec<Result<T,E>>` 能 collect 成 `Result<Vec<T>, E>`——  
> 有一个失败就整体失败。

```rust
let nums: Result<Vec<i64>, _> = ["1", "2", "x"]
    .into_iter()
    .map(|s| s.parse::<i64>())
    .collect();   // Err（"x" 解析失败）
```

约等于 Go「循环 + 遇 err 就 return」整段模板，Rust 一行完成。  
数据处理、批量解析时极其好用。

----

# 何时还用 for

> 迭代器链不是教条。这些情况 `for` 往往更清晰：

- 循环体有多步 **副作用**（打日志、改多个外部变量）；  
- 复杂条件下 **提前 break**，状态不好折进 `fold`；  
- 读起来「链拧成麻花」，同事（和三个月后的你）骂娘。

经验口诀：

> **「数据进、数据出」用链；「过程控制」用 for。**

```rust
let mut total = 0;
for n in 1..=10 {
    if n % 2 == 0 { continue; }
    total += n;
    if total > 15 { break; }
}
```

----

# 和所有权的咬合

> 链上每个闭包都在捕获/借用——规则和 [《函数与闭包》](functions-closures.md) 相同。

常见点：

- `iter()` 后闭包里是 `&T`，数字常要 `copied()` / `cloned()` 才能当值用；  
- `into_iter()` 吃掉集合，换「拥有的 `T`」进链；  
- 别在 `iter()` 持有借用时去 `push` 同一个 `Vec`。

搞不清报错时：先看是不是 `for x in v` 吃掉了集合，再看闭包有没有违规借用。

----

# 动手实验

```bash
cd code
cargo run -p lang-iterators
```

1. 把「挑长度 > 3 → 转大写 → 取前 5」分别写成 for 版和链版，确认输出一致；  
2. 故意 `for x in v {}` 后再用 `v`，读 E0382，改成 `&v`；  
3. 同一 `(1..=6)` 分别 collect 成 `Vec`、`HashSet`、`String`（先 map 成 char）；  
4. 用 `parse` + collect `Result` 处理含非法数字的列表；  
5. 读 `fold` 文档，手写一个「求最大值」的 fold，再和 `max` 对照。

----

# 三句话带走

1. **三种入口**：`iter` / `iter_mut` / `into_iter`；`for x in v` 会吃掉 `v`。  
2. **链惰性 + 零成本**：末端才执行，性能对手写循环。  
3. **`collect` 看左边类型**；`Vec<Result>` → `Result<Vec>` 是必会名招。

下一章：[《泛型与 trait bound》](generics.md)。  
字符串切片忘了回 [《字符串、数组与切片》](strings-slices.md)。

----

# 附：本章生词表

- **`iter` / `iter_mut` / `into_iter`**：产出 `&T` / `&mut T` / `T`。  
- **适配器（adapter）**：`map`/`filter` 等，惰性变换迭代器。  
- **消费者（consumer）**：`collect`/`sum`/`for_each` 等末端方法，驱动执行。  
- **惰性**：搭好不跑，消费才跑。  
- **`filter_map`**：`Option` 版 map+filter。  
- **`fold`**：万能累积，≈ reduce。  
- **`collect` / turbofish**：目标类型驱动收集；`collect::<Vec<_>>()`。  
- **零成本抽象**：高阶写法编译后与手写循环同级性能。  
- **`enumerate` / `zip`**：带下标 / 两条并排。
