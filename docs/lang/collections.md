# 集合谁对应谁

> 代码：[`code/lang-collections/`](../../code/lang-collections/)　  
> 运行：在 `code/` 目录下执行 `cargo run -p lang-collections`

> 前置：[《所有权与借用》](ownership.md)、 [《字符串、数组与切片》](strings-slices.md)。  
> Go 日常两件套：`[]T` 和 `map[K]V`。Rust 里拥有版主力是 **`Vec<T>`** 和 **`HashMap<K,V>`**。  
> 这一章把增删改查、`entry` 名场面、以及周边的 Set/有序 Map/双端队列过一遍。

借用视图 `&[T]`、字符串细节见上一章；  
「迭代器链代替 for」见 [《迭代器》](iterators.md)。

| Go | Rust | 备注 |
| --- | --- | --- |
| `[]T`（slice） | `Vec<T>`（拥有）/ `&[T]`（视图） | 本章聚焦 Vec |
| `map[K]V` | `HashMap<K, V>` | K 要 `Eq + Hash` |
| 有序 map | `BTreeMap<K, V>` | 按 key 排序遍历 |
| `map[T]bool` 当 set | `HashSet<T>` | 真正的集合 |
| 双端队列 | `VecDeque<T>` | 两端 O(1) |

----

# Vec 常用操作

> `Vec` ≈ Go 的 slice，绝大多数操作直觉一致。

```rust
let mut v = vec![3, 1, 2];
v.push(4);                    // append
v.pop();                      // 取尾，返回 Option
v.sort();                     // 排序
v.sort_by_key(|u| u.age);     // 按 key 排序
v.retain(|x| *x > 1);         // 原地过滤（Go 要手写）
v.contains(&2);
v.first(); v.last();          // Option，越界不 panic
let (l, r) = v.split_at(1);   // 借用切两半（见字符串/切片章）
```

造法：

```rust
let a = vec![1, 2, 3];
let b = vec![0i32; 5];        // 5 个 0
let c: Vec<i32> = (0..3).collect();  // 从迭代器收集，见迭代器章
```

扩容直觉和 Go 一样：容量不够时换更大缓冲、搬走元素——  
所以持有元素借用/`&[T]` 时不能再 `push`（借用检查器会拦）。

----

# 下标越界会 panic

> `v[i]` 越界 → **panic**（和 Go 一样）。  
> 业务代码优先 `v.get(i)` → `Option`，好映射成 404 / 错误，而不是崩服务。

```rust
let v = vec!["甲", "乙"];
match v.get(1) {
    Some(x) => println!("{x}"),
    None => println!("没有"),
}
// v[99];   // 💥 别在服务里这么干
```

[《读写接口与错误处理》](../http/rest.md) 里按索引取元素返回 404，靠的就是 `get`。

----

# HashMap 没有零值

> Go：`m["b"]` 可能默默给你零值，你分不清「没有」还是「就是 0」。  
> Rust：`get` 返回 `Option`——可能没有，必须显式面对。

```rust
use std::collections::HashMap;

let mut m: HashMap<String, i64> = HashMap::new();
m.insert("a".into(), 1);

match m.get("b") {
    Some(v) => println!("有: {v}"),
    None => println!("没有"),
}

// 想要「没有就当 0」？亲口说：
let v = m.get("b").copied().unwrap_or(0);
```

Key 的要求：实现 `Eq + Hash`。自己的结构体通常 `#[derive(Eq, PartialEq, Hash)]`。  
字符串作 key 时，`get` 可以传 `&str`（不必先建成 `String`）——靠 `Borrow` 机制。

遍历顺序 **不稳定**（和 Go `range` map 一样）；要稳定顺序用 `BTreeMap` 或先收集再排序。

----

# entry 一行搞定

> Go「没有就初始化再改」经典三行，Rust 用 **entry API** 一行完成——天天用。

```rust
// Go: if _, ok := m[k]; !ok { m[k] = 0 }; m[k]++
*m.entry("a".into()).or_insert(0) += 1;   // 词频统计名场面

// 分组：没有就先塞空 Vec，再 push
m.entry("list".into()).or_insert_with(Vec::new).push(item);
```

读法：`entry(k)` 拿到这个位子的句柄；  
`or_insert(v)` = 不存在就放 `v`，然后给我里面值的 `&mut`；  
`or_insert_with(闭包)` = 惰性构造默认值（构造贵时用）。

还有 `and_modify(|v| …)`：存在才改，常和 `or_insert` 组合。

----

# HashSet 真集合

> Go 常用 `map[T]struct{}` 或 `map[T]bool` 模拟集合；Rust 直接给 `HashSet<T>`。

```rust
use std::collections::HashSet;

let mut set = HashSet::from(["a", "b", "a"]);
set.insert("c");
set.contains("b");
```

集合运算：`union` / `intersection` / `difference` 等（返回迭代器）。  
要有序集合 → `BTreeSet`。

----

# BTreeMap 有序

> 需要「按 key 排序遍历」时用 `BTreeMap`。  
> Go 里往往是：收集 keys → sort → 再取。

```rust
use std::collections::BTreeMap;

let mut m = BTreeMap::new();
m.insert("c", 3);
m.insert("a", 1);
m.insert("b", 2);
for (k, v) in &m {   // a, b, c
    println!("{k}={v}");
}
```

代价：操作一般比 HashMap 慢一截（树 vs 哈希），按需选用。

----

# VecDeque 双端队列

> 两端都要高频 `push`/`pop` 时，用 `VecDeque`（`Vec` 只在尾部摊还 O(1)）。

```rust
use std::collections::VecDeque;

let mut q = VecDeque::from([2, 3]);
q.push_front(1);
q.push_back(4);
q.pop_front();
```

对照 Go：`container/list` 或自己环形缓冲；这里标准库直接给。

----

# 动手实验

```bash
cd code
cargo run -p lang-collections
```

1. **词频统计**：`split_whitespace` + `entry`，再按次数排序输出；  
2. **分组**：把 `(分类, 名字)` 列表收成 `HashMap<&str, Vec<&str>>`；  
3. 对 `Vec` 用 `get` 和 `[]` 各取一次越界，对比行为；  
4. 同一批数据分别插入 `HashMap` 与 `BTreeMap`，打印遍历顺序差异。

----

# 三句话带走

1. **`Vec` ≈ Go slice**：越界 `[]` 会 panic，业务优先 `get` → `Option`。  
2. **`HashMap` 没有零值**：`get` 返回 `Option`；「没有就初始化」用 **entry**。  
3. **周边选型**：去重用 `HashSet`，要有序用 `BTreeMap`，双端用 `VecDeque`。

下一章：[《迭代器》](iterators.md) ——用链代替手写 for 的主力习惯。

----

# 附：本章生词表

- **`Vec<T>`**：可增长的拥有型动态数组。  
- **`HashMap<K,V>`**：哈希表；K 需 `Eq + Hash`。  
- **`entry` / `or_insert` / `or_insert_with`**：按 key 占位并得到 `&mut V`。  
- **`HashSet<T>`**：哈希集合。  
- **`BTreeMap` / `BTreeSet`**：有序 map / 有序集合。  
- **`VecDeque<T>`**：双端队列。  
- **`retain`**：按谓词原地过滤 Vec。  
- **`Borrow`**：让 `HashMap<String,_>` 能用 `&str` 查找的机制（了解即可）。
