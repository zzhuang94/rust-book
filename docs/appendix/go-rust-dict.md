# 怎么用这本词典

> 写 Rust 时脑子里冒出「这个 Go 里怎么写来着」，来这里按主题查。每条给出最惯用的对应写法；「注」列标的是细讲章节。不求穷尽，求高频。

和成体系的 [《Go → Rust 语言对照》](../start/go-vs-rust.md) 互补：那一章讲「为什么不同」，这本词典重在「随手查怎么写」。

----

# 变量类型与零值

| Go | Rust | 注 |
| --- | --- | --- |
| `var x int` | `let x: i64 = 0;` | Rust 无零值：必须初始化才能用 |
| `x := 5` | `let x = 5;` | 推断同款；Rust 默认 **不可变** |
| `x = 6`（重新赋值） | `let mut x = 5; x = 6;` | 要改必须 mut |
| `const N = 10` | `const N: i64 = 10;` | Rust 的 const 必须标类型 |
| `int / int64 / int32` | `i64 / i64 / i32`（还有 usize 做下标） | 无隐式转换，跨类型用 `as` 或 `try_into` |
| `float64` | `f64` | |
| `byte / rune` | `u8 / char` | char 是 4 字节 Unicode 标量 |
| `interface{}` / `any` | 尽量别；泛型 `<T>` 或 `Box<dyn Trait>` | 类型系统与 trait |
| `type UserID int64` | `struct UserId(u64);`（newtype） | 类型系统与 trait；Rust 版不能和 u64 混算，更安全 |
| nil | **不存在**；`Option<T>` 的 None | 语法底座 |
| 零值可用（map/slice 的 nil 可读） | 不存在；`Vec::new()` / `HashMap::new()` 显式建 | |

----

# 流程控制

| Go | Rust | 注 |
| --- | --- | --- |
| `if x > 0 { }` | `if x > 0 { }` | 同款，条件不加括号 |
| `if v, ok := m[k]; ok { }` | `if let Some(v) = m.get(k) { }` | 模式匹配 |
| `switch x { case 1: ... }` | `match x { 1 => ..., _ => ... }` | match 必须穷尽；模式匹配 |
| `switch { case x>0: }` 无表达式 | 直接 if/else 或 `match () { _ if x>0 => ... }` | 前者更常用 |
| `for i := 0; i < n; i++` | `for i in 0..n` | |
| `for i, v := range slice` | `for (i, v) in v.iter().enumerate()` | 集合迭代器 |
| `for k, v := range m` | `for (k, v) in &m` | |
| `for { }`（死循环） | `loop { }` | loop 可以 `break 值` 当表达式 |
| `break LabelName` | `break 'label` | 标签带撇号：`'outer: loop { ... break 'outer; }` |
| `defer f()` | 无直接对应：RAII/Drop，或 scopeguard crate | 所有权 |

----

# 函数方法闭包

| Go | Rust | 注 |
| --- | --- | --- |
| `func f(a int) int { return a }` | `fn f(a: i64) -> i64 { a }` | 尾表达式即返回值 |
| 多返回值 `(int, error)` | 元组 `(i64, String)` 或 `Result<i64, E>` | 错误场景一律 Result |
| 命名返回值 | 不存在 | |
| 可变参数 `...int` | 无直接对应；传 `&[i64]` 或宏 | println! 那类是宏在办 |
| `func (u *User) Rename(n string)` | `fn rename(&mut self, n: &str)`（impl 块中） | 类型系统与 trait 三种 self |
| 匿名函数 `func(x int) int { ... }` | 闭包 `\|x: i64\| -> i64 { ... }`（常可省类型） | 语法底座 |
| 闭包捕获外部变量 | 默认借用；跨线程/任务加 `move` | 所有权 |
| 函数当值传递 | 同款：`fn` 名字即值；闭包看 Fn/FnMut/FnOnce | |
| `init()` | 不存在；`LazyLock` / `OnceCell` 惰性初始化 | 通知与热更新 |

----

# 结构体与接口

| Go | Rust | 注 |
| --- | --- | --- |
| `type User struct { Name string }` | `struct User { name: String }` | 可见性用 pub 不用大小写 |
| 大写导出 / 小写私有 | `pub` / 默认私有 | 模块与可见性 |
| `u := User{Name: "a"}` | `let u = User { name: "a".into() };` | |
| 嵌入（匿名字段）复用 | 组合 + 手写转发 / trait 默认方法 | 类型系统与 trait，无自动方法提升 |
| `interface { Read(...) }` 隐式实现 | `trait` + **显式** `impl X for Y` | 语法底座 |
| interface 变量装任何实现 | `Box<dyn Trait>`（动态）或泛型（静态，更常用） | 泛型 |
| 类型断言 `v.(Concrete)` | `downcast_ref`（Any/anyhow 场景）；一般用 enum 替代 | 错误处理 |
| `String() string`（Stringer） | `impl Display` → 自动获得 `.to_string()` | 类型系统与 trait |
| `json:"name"` tag | `#[derive(Serialize, Deserialize)]` + `#[serde(rename = "name")]` | axum |

----

# 错误处理

> 整篇细讲见 [《通用错误处理》](../lang/error-handling.md)。

| Go | Rust |
| --- | --- |
| `if err != nil { return err }` | `?` |
| `fmt.Errorf("读配置: %w", err)` | `.context("读配置")`（anyhow） |
| `errors.New("x")` | `anyhow!("x")`；库里定义 thiserror 变体 |
| `errors.Is(err, ErrX)` | match 变体 / `err.is::<E>()` |
| `errors.As(err, &t)` | `err.downcast_ref::<E>()` |
| 自定义 error 类型 | `#[derive(thiserror::Error)]` 的 enum |
| `panic(...)` / `recover` | `panic!` / `catch_unwind`（仅框架边界用） |
| `log.Fatal(err)` | `main -> anyhow::Result<()>` + `?`（自动打链退出） |

----

# 集合操作

> 细讲见 [《字符串、数组与切片》](../lang/strings-slices.md)、  
> [《集合：Vec 与 HashMap》](../lang/collections.md)、 [《迭代器》](../lang/iterators.md)。

| Go | Rust |
| --- | --- |
| `append(v, x)` | `v.push(x)` |
| `v = append(v, other...)` | `v.extend(other)` |
| `len(v)` / `cap(v)` | `v.len()` / `v.capacity()` |
| `v[1:3]` | `&v[1..3]`（是借用！） |
| `copy(dst, src)` | `dst.copy_from_slice(&src)` / `to_vec()` |
| `sort.Slice(v, less)` | `v.sort_by(...)` / `sort_by_key(...)` |
| `slices.Contains(v, x)` | `v.contains(&x)` |
| 手写 filter 循环 | `v.retain(...)`（原地）/ `.iter().filter(...).collect()` |
| `m[k] = v` | `m.insert(k, v)` |
| `v, ok := m[k]` | `m.get(&k)` → Option |
| `delete(m, k)` | `m.remove(&k)` |
| 没有就初始化再改 | `*m.entry(k).or_insert(0) += 1` |
| `make([]T, 0, 100)` | `Vec::with_capacity(100)` |

----

# 字符串

| Go | Rust |
| --- | --- |
| `s := "abc"` | `let s = "abc";`（&str）；要拥有 `.to_string()` |
| `s1 + s2` | `format!("{s1}{s2}")` 或 `s1 + &s2`（s1 须 String） |
| `fmt.Sprintf("%d-%s", n, s)` | `format!("{n}-{s}")` |
| `strconv.Atoi(s)` | `s.parse::<i64>()`（Result） |
| `strconv.Itoa(n)` | `n.to_string()` |
| `strings.Split(s, ",")` | `s.split(',')`（迭代器） |
| `strings.Join(v, ",")` | `v.join(",")` |
| `strings.TrimSpace(s)` | `s.trim()` |
| `strings.Contains/HasPrefix` | `s.contains(..)/starts_with(..)` |
| `[]byte(s)` / `string(b)` | `s.as_bytes()` / `String::from_utf8(b)?` |
| `for _, r := range s`（按 rune） | `for c in s.chars()` |
| `filepath.Join(a, b)` | `PathBuf::from(a).join(b)` |

----

# 并发

> 细讲散在异步主线各章。

| Go | Rust（tokio） |
| --- | --- |
| `go f()` | `tokio::spawn(async move { f().await })` |
| `time.Sleep(d)` | `tokio::time::sleep(d).await`（async 里 **禁** 用 std sleep） |
| `sync.WaitGroup` | 收集 JoinHandle 逐个 await / `JoinSet` |
| `errgroup.WithContext` | `try_join!` / JoinSet+abort |
| `errgroup.SetLimit(3)` | `Semaphore::new(3)` |
| `ch := make(chan T, n)` | `mpsc::channel::<T>(n)` |
| `v := <-ch` / `ch <- v` | `rx.recv().await` / `tx.send(v).await` |
| `close(ch)` | 不存在：drop 所有 tx 即关闭 |
| `for v := range ch` | `while let Some(v) = rx.recv().await` |
| `select { case ... }` | `tokio::select! { ... }` |
| `context.WithCancel` | `CancellationToken` |
| `context.WithTimeout` | `timeout(d, fut)` |
| `sync.Mutex` + 变量 | `Arc<Mutex<T>>`（数据在锁里） |
| `sync.RWMutex` | `Arc<RwLock<T>>` |
| `atomic.AddInt64` | `AtomicU64::fetch_add(1, Ordering::Relaxed)` |
| `sync.Once` | `OnceCell`（可 await）/ `LazyLock` |
| `sync.Cond` / 信号 chan | `Notify` |
| 配置热更新（atomic.Pointer+chan） | `watch::channel` |
| `runtime.NumCPU()` | `std::thread::available_parallelism()` |

----

# 时间

| Go | Rust |
| --- | --- |
| `time.Now()` | `std::time::Instant::now()`（测耗时）/ `chrono::Local::now()`（墙钟） |
| `time.Since(t)` | `t.elapsed()` |
| `5 * time.Second` | `Duration::from_secs(5)` |
| `time.NewTicker(d)` | `tokio::time::interval(d)` |
| `t.Format("2006-01-02")` | `now.format("%Y-%m-%d")`（chrono，labkit 里就是它） |

----

# JSON 与序列化

| Go | Rust（serde + serde_json） |
| --- | --- |
| `json.Marshal(v)` | `serde_json::to_string(&v)?`（v 需 derive Serialize） |
| `json.Unmarshal(b, &v)` | `let v: T = serde_json::from_str(s)?;` |
| struct tag 控制字段 | `#[serde(rename/skip/default/rename_all = "camelCase")]` |
| `map[string]any` 临时 JSON | `serde_json::Value` / `json!({...})` 宏 |
| 缺字段 → 零值 | 缺字段 → 报错；想可选用 `Option<T>` 或 `#[serde(default)]` |

----

# 文件与 IO

| Go | Rust |
| --- | --- |
| `os.ReadFile(p)` | `std::fs::read_to_string(p)?`（异步：`tokio::fs::...await`） |
| `os.WriteFile(p, b, 0644)` | `std::fs::write(p, b)?` |
| `os.Open` + bufio.Scanner 按行 | `BufReader::new(File::open(p)?).lines()` |
| `os.Getenv("K")` | `std::env::var("K")`（Result，没设置是 Err 不是空串） |
| `os.Args` | `std::env::args()`；正经 CLI 用 clap |
| `defer f.Close()` | 不用写：File 的 Drop 自动关 |

----

# 工程与工具链

| Go | Rust |
| --- | --- |
| `go mod init` / go.mod | `cargo new` / Cargo.toml |
| `go build` / `go run` | `cargo build` / `cargo run`（发布加 `--release`） |
| `go test` | `cargo test` |
| `gofmt` | `cargo fmt` |
| `go vet` + staticcheck | `cargo clippy`（更强更唠叨，建议常开） |
| godoc | `cargo doc --open` |
| `go get pkg` | `cargo add pkg` |
| go.work / 多模块 | Cargo workspace（本教程就是） |
| internal/ 包 | `pub(crate)` 可见性 |
| build tags | `#[cfg(...)]` / features |
| 单二进制部署 | 同款优势；交叉编译看 [《构建与部署》](../engineering/build-deploy.md) |

----

# 三类没有对照的

> 词典给的是「最短路径」写法， **为什么这样写** 去对应章找。有些概念别硬找 Go 等价物。

三类 Rust 概念，直接按正文建立新心智，别硬找 Go 对照：**所有权/借用**（[《所有权与借用》](../lang/ownership.md)）、 **生命周期**（[《生命周期》](../lang/lifetimes.md)）、  
**trait 的静态分发**（[《泛型与 trait bound》](../lang/generics.md)）、 **Pin**（[《Go 并发实现（GMP）》](../concurrency/go-gmp.md)）。

反过来，两个 Go 概念在 Rust 里刻意不存在：**nil**（用 Option）和 **零值**（必须显式初始化/Default）——想念它们的时候，  
就是该重读 [《Rust 语法底座》](../start/syntax-primer.md) 「没有 nil」那几节的时候。
