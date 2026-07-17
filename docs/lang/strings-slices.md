# 文本与连续内存

> 代码：[`code/lang-strings-slices/`](../../code/lang-strings-slices/)　  
> 运行：在 `code/` 目录下执行 `cargo run -p lang-strings-slices`

> 前置：[《所有权与借用》](ownership.md)、 [《基础类型》](basics.md)。  
> Go 里 `string` 和 `[]T` 你用到麻木；Rust 把「拥有」和「只看一段」拆得更清楚——  
> `String` vs `&str`，`Vec` vs `&[T]`，定长数组 `[T; N]`。  
> 这一章把日常读写文本、切一段缓冲的手感钉死。

`Vec` / `HashMap` 见 [《集合：Vec 与 HashMap》](collections.md)；  
迭代器链见 [《迭代器》](iterators.md)。

----

# String 与 &str

> Rust 把字符串拆成两个类型：拥有者 `String`，只读视图 `&str`。

| | `String` | `&str` |
| --- | --- | --- |
| 是什么 | 拥有、可增长的堆上 UTF-8 | 指向某段 UTF-8 的借用 |
| Go 对照 | 近似 `strings.Builder` 的结果 | ≈ Go 的 `string`（只读字节序列） |
| 函数参数 | 要拿走/存进结构体时用 | **默认用它** |
| 互转 | `s.as_str()` / `&s` | `s.to_string()` / `s.to_owned()` |

```rust
let owned = String::from("拥有");
let view: &str = &owned;       // String → &str（降级借用）
let literal: &str = "字面量";  // &'static str
let again = view.to_string();  // &str → String
```

一句话：**函数参数默认收 `&str`**（字面量和 `String` 都能传）；  
需要「拿走并保存」时再收 `String`。

----

# UTF-8 的硬规矩

> 这是 Go 程序员第二道坎。Rust 字符串 **保证** 合法 UTF-8；  
> `len()` 是 **字节数**，不是「几个字」。

```rust
let s = "你好a";
s.len();                 // 7（3+3+1），不是 3！
// s[0];                 // ❌ 不许按下标取字符
s.chars().count();       // 3 —— 字符数要 O(n) 数
s.chars().nth(1);        // Some('好')
&s[0..3];                // "你" —— 字节切片，必须落在字符边界
// &s[0..4];             // 💥 panic：切在汉字中间
```

对照 Go：底层模型其实一样（UTF-8 字节序列），`len` 也是字节数。  
Rust 把「当成字符数组下标乱取」的口子焊死了。牢记：

1. **切片按字节，但必须落在字符边界**；  
2. **数「几个字」用 `chars().count()`**。

----

# 字符串操作对照

> Go `strings` / `strconv` → Rust 方法。注意 `split` 返回 **迭代器**。

| Go | Rust |
| --- | --- |
| `Contains` | `s.contains(..)` |
| `HasPrefix` / `HasSuffix` | `starts_with` / `ends_with` |
| `Split` | `s.split(',')`（迭代器） |
| `TrimSpace` | `s.trim()` |
| `ToUpper` / `ToLower` | `to_uppercase` / `to_lowercase` |
| `Replace` | `s.replace(a, b)` |
| `Join` | `parts.join(",")` |
| `Atoi` / `Itoa` | `s.parse::<i64>()` / `n.to_string()` |
| `fmt.Sprintf` | `format!` |
| `strings.Builder` | `String` + `push_str` |

因为 `split` 是迭代器，切开后可直接接链（细节见 [《迭代器》](iterators.md)）：

```rust
let nums: Vec<i64> = "1, 2, , 3".split(',')
    .map(|p| p.trim())
    .filter(|p| !p.is_empty())
    .map(|p| p.parse().unwrap())
    .collect();
```

----

# 定长数组

> `[T; N]`：长度 `N` 写进类型。`[i32; 3]` 和 `[i32; 4]` 是两种类型。  
> 对照 Go 的 `[3]int`；日常变长数据更常用后面的 `Vec`。

```rust
let nums: [i32; 3] = [10, 20, 30];
let zeros = [0i32; 4];           // 4 个 0
let x = nums[1];                 // 越界 → panic
let y = nums.get(99);            // None，不崩
```

元素是 `Copy` 时，数组可以整体拷贝。  
要「任意长、可增长」→ [《集合：Vec 与 HashMap》](collections.md)。

----

# 切片是一段视图

> `&[T]` / `&mut [T]`：指向连续元素某一段的借用（胖指针 = 起点 + 长度）。  
> [《所有权与借用》](ownership.md) 已铺垫：切片不拷贝数据，受借用铁律管辖。

```rust
let mut v = vec![1, 2, 3, 4, 5];
let mid: &[i32] = &v[1..4];          // [2,3,4]
let (l, r) = v.split_at(2);          // 切成两半借用
let tail: &mut [i32] = &mut v[3..];  // 可变切一段
tail[0] = 99;

let arr = [7, 8, 9];
let s: &[i32] = &arr[1..];           // 数组也能出切片
```

关系记一张表：

| 拥有 | 借用视图 |
| --- | --- |
| `String` | `&str` |
| `Vec<T>` | `&[T]` |
| `[T; N]` | `&[T]`（可切一段） |
| `PathBuf` | `Path` |

函数参数若只需读一段连续数据，优先 `&[T]` / `&str`，调用方用 `Vec`/`String`/数组/字面量都能传。

----

# 路径别用 String

> 拼文件路径别 `format!("{dir}/{name}")`。用 `Path` / `PathBuf`——  
> 跨平台分隔符，还能容忍「文件名不是合法 UTF-8」。

```rust
use std::path::PathBuf;

let mut p = PathBuf::from("/data");
p.push("logs");
p.push("app.log");     // ≈ filepath.Join
p.extension();         // Some("log")
```

`Path` / `PathBuf` ≈ `&str` / `String`（借用 vs 拥有）。

----

# 动手实验

```bash
cd code
cargo run -p lang-strings-slices
```

1. 对 `"你好世界"` 执行 `&s[0..4]`，读 panic 里「不是字符边界」；  
2. 写一个只收 `&str` 的函数，分别传入字面量和 `String`；  
3. 用 `split_at` 把 `Vec` 切成两半只读视图，尝试在持有视图时 `push`，看借用报错；  
4. 用 `PathBuf` 拼一条相对路径，在 Windows / Linux 上各跑一次看分隔符。

----

# 三句话带走

1. **参数默认 `&str` / `&[T]`**，要存起来再收 `String` / `Vec`。  
2. **字符串是 UTF-8 字节序列**：`len` 是字节数，切片落在字符边界，数「字」用 `chars()`。  
3. **路径用 `PathBuf`**，别拿 `String` 手拼。

下一章：[《集合：Vec 与 HashMap》](collections.md)。

----

# 附：本章生词表

- **`String` / `&str`**：拥有的字符串 / UTF-8 借用视图。  
- **字符边界**：UTF-8 多字节字符的起止位置；切片必须对齐。  
- **`chars()`**：按 Unicode 标量值迭代。  
- **定长数组 `[T; N]`**：长度属于类型的数组。  
- **切片 `&[T]`**：连续元素的借用视图（胖指针）。  
- **`split_at`**：把切片借成左右两半。  
- **`Path` / `PathBuf`**：路径的借用/拥有类型。  
- **`parse::<T>()`**：字符串解析为 `T`，返回 `Result`。
