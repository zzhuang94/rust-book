# 并发的两条路

> 代码：[`code/concurrency-threads/`](../../code/concurrency-threads/)　  
> 运行：在 `code/` 目录下执行 `cargo run -p concurrency-threads`  
> （全部标准库、不需要 tokio；片段也可单独 `cargo new demo` 粘进 `main.rs` 跑）

> 前置：[《进程与线程》](../os/process-thread.md)（线程/调度/阻塞）、 [《Rust 语法底座》](../start/syntax-primer.md)（闭包/move/Option/Result）。

Rust 写并发有两条路，对应 [《进程与线程》](../os/process-thread.md) 那张三种并发单元表的两列：

| | 路线一：OS 线程（本篇） | 路线二：异步任务（[《async 基础》](../async/basics.md) 起） |
| --- | --- | --- |
| 工具 | `std::thread`（标准库，零依赖） | tokio |
| 并发单元成本 | 每线程 ~8MB 栈、微秒级切换 | 每任务几百字节、纳秒级切换 |
| 适合 | CPU 密集并行、并发量可控（几个~几百） | 海量 IO 并发（连接数 ≫ 核数） |
| Go 对照 | 没有直接对应（goroutine 更像右边） | goroutine 的常规用法 |

为什么要先学线程这条路：

- 它是 **地基**：Tokio 的多线程运行时底下就是一个 `std::thread` 线程池，`spawn_blocking` 丢过去的闭包就跑在这种线程上；
- 它 **够简单**：没有 Future、没有 await，让你把注意力全放在「所有权如何管住并发」上—— [《共享状态：Arc / RwLock》](../async/shared-state.md) 的 `Arc`/`Mutex`/`Send`/`Sync` 在这里全部原样适用，  
  先在简单世界练熟；
- 它在很多场景 **就是正确答案**：CPU 密集的并行计算（图像处理、数据加工），用线程比用异步更直接。

----

# spawn 与 join

> 第一个线程：`spawn` 开一个，`join` 等它结束并拿回结果。对照 Go 的 `go` + `WaitGroup`。

```rust
use std::thread;
use std::time::Duration;

fn main() {
    // spawn：创建一个 OS 线程去执行闭包，立即返回句柄（不等它跑完）
    // 对照 Go：go func(){ ... }()，但 Rust 给你一个 JoinHandle
    let handle = thread::spawn(|| {
        for i in 1..=3 {
            println!("  子线程: 第 {i} 步");
            thread::sleep(Duration::from_millis(100)); // OS 线程里 sleep 是正当的（只睡自己）
        }
        42 // 闭包的返回值 = 线程的"结果"
    });

    println!("主线程：子线程在后台跑，我继续干别的");

    // join：等线程结束，拿回它的返回值
    // 对照 Go：WaitGroup.Wait() + 用 channel 传结果，这里一个调用全包了
    let result = handle.join().unwrap();
    println!("主线程：子线程返回了 {result}");
}
```

逐点拆解：

- `thread::spawn(闭包)`：真的向 OS 申请了一个新线程（[《进程与线程》](../os/process-thread.md)：  
  独立的 8MB 栈 + 上下文），闭包在里面执行；
- `JoinHandle<T>`：线程的遥控器，`join()` = 阻塞等它结束并取回返回值。和 [《Tokio 运行时》](../async/tokio.md) 的 JoinHandle 长得像不是巧合——tokio 就是照着它设计的，  
  区别是 tokio 版用 `.await` 等（挂起任务），这里用 `.join()` 等（阻塞线程）；
- `join().unwrap()` 的 unwrap 在解什么：`join` 返回 `Result<T, _>`—— **线程 panic 了就是 Err**。  
  对照 Go：goroutine panic 直接炸整个进程；Rust 线程 panic 默认只死自己，父线程通过 join 感知（这点比 Go 温和）。

**⚠️ 新手第一坑：main 退出 = 全体死亡。** 把上面 `handle.join()` 那两行删掉，子线程的输出很可能只打一半甚至没有——main 函数返回时 **进程直接结束，所有线程原地消失**（不会等它们）。Go 完全一样（main 退出不等 goroutine）。所以要么 join， 
要么明确知道自己在做「后台守护」。

----

# 数据带进线程

> 线程可能比创建它的函数活得久，所以借用检查器盯得很紧。想把数据带进线程，用 `move` 转移所有权。

```rust
use std::thread;

fn main() {
    let data = vec![1, 2, 3];

    // 写法一：借用 —— 编译不过！
    // let h = thread::spawn(|| println!("{data:?}"));
    //   ↑ error[E0373]: closure may outlive the current function, but it borrows `data`
    //   人话：线程可能比 main 的这段栈活得久，借用会悬垂

    // 写法二：move —— 把所有权搬进线程，编译通过
    let h = thread::spawn(move || println!("子线程拿到: {data:?}"));
    h.join().unwrap();

    // println!("{data:?}"); // ← 放开也编译不过：data 已经搬走了（E0382）
}
```

对照 Go：`go func(){ fmt.Println(data) }()` 随手捕获就行（GC 保证 data 活着）。  
Rust 用 `move` + 所有权在 **编译期** 回答「data 归谁、活多久」。想让 **多个** 线程共享同一份 data？  
那就是下一节的 `Arc`。

----

# 共享靠 Arc Mutex

> 多线程改同一个计数器——`Arc` 负责共享、`Mutex` 负责互斥。这套和 [《共享状态：Arc / RwLock》](../async/shared-state.md) 的异步版一字不差。

```rust
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Arc：让 4 个线程共享同一个计数器（引用计数句柄）
    // Mutex：把数据锁在里面，不 lock 拿不到 —— "忘了加锁"不存在
    let counter = Arc::new(Mutex::new(0u64));

    let mut handles = Vec::new();
    for _ in 0..4 {
        let counter = Arc::clone(&counter); // 计数 +1，不复制数据
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                *counter.lock().unwrap() += 1; // 语句结束 guard drop = 解锁
            }
        }));
    }
    for h in handles {
        h.join().unwrap(); // ≈ WaitGroup.Wait()
    }
    println!("结果 = {}（精确 4000）", *counter.lock().unwrap());
}
```

几个关键点（[《共享状态：Arc / RwLock》](../async/shared-state.md) 都有完整展开，这里点到）：

- 为什么必须 `Arc`：`thread::spawn` 和 `tokio::spawn` 一样要求闭包 `'static` + 捕获物 `Send`—— **`Send`/`Sync` 体检对 OS 线程同样生效**，  
  毕竟这套机制本来就是为线程设计的，异步任务只是沿用；
- 把 `Mutex` 拿掉直接 `+= 1`？编译不过（`Arc` 里的数据不可变，需要内部可变性）；用 `unsafe` 强行绕过？  
  那就是 [《进程与线程》](../os/process-thread.md) 说的数据竞争，结果随机少——Go 里 `-race` 才能抓到的问题，  
  Rust 在这里根本写不出来；
- 对照 Go：`var mu sync.Mutex; var n int` + `go func(){ mu.Lock(); n++; mu.Unlock() }()`——形状一样，  
  区别是 Go 的锁和数据靠自觉配对，Rust 的数据锁在 `Mutex` 里面。

原子量版本同样照搬：把 `Mutex<u64>` 换成 `AtomicU64`、`lock()` 换成 `fetch_add(1, Ordering::Relaxed)`，  
无锁且同样精确。

----

# 通信靠 mpsc

> 不想共享内存？让线程之间 **发消息** ——`std::sync::mpsc`，是 [《Tokio 运行时》](../async/tokio.md) mpsc 通道的同步祖先。

```rust
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn main() {
    let (tx, rx) = mpsc::channel::<String>(); // 无界通道（还有 sync_channel(n) 是有界带背压的）

    for id in 1..=2 {
        let tx = tx.clone(); // 多生产者
        thread::spawn(move || {
            for n in 1..=3 {
                tx.send(format!("生产者{id} 的第 {n} 条")).unwrap();
                thread::sleep(Duration::from_millis(50));
            }
            // 线程结束，tx 被 drop
        });
    }
    drop(tx); // 丢掉 main 手里的原始 tx，否则通道永远不关

    // rx 是迭代器！所有 tx 都 drop 后迭代自然结束 ≈ Go 的 for msg := range ch
    for msg in rx {
        println!("收到: {msg}");
    }
    println!("通道关闭");
}
```

和 tokio mpsc 的差异对照（将来切到异步时不迷路）：

| | `std::sync::mpsc` | `tokio::sync::mpsc` |
| --- | --- | --- |
| `recv` 等消息时 | **阻塞线程** | 挂起任务（`.await`） |
| 容量 | `channel()` 无界 / `sync_channel(n)` 有界 | `channel(n)` 有界（推荐）/ `unbounded_channel()` |
| 收消息循环 | `for msg in rx`（rx 实现了 Iterator） | `while let Some(m) = rx.recv().await` |
| 关闭语义 | 相同：所有 tx drop = 关闭 | 相同 |

Go 对照：就是 `ch := make(chan string)` 那套，连「多生产者 clone、全部退出通道才关」的心智都一致——只是 Go 靠 `close(ch)` 显式关，  
Rust 靠 tx 的所有权消失。

----

# 作用域线程

> `thread::spawn` 要求 `'static`（不能借栈上数据），但很多场景明明是「开几个线程处理 **当前函数里** 的数据，  
> 处理完一起收工」。`thread::scope` 专治这个。

```rust
use std::thread;

fn main() {
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let mid = data.len() / 2;
    let (left, right) = data.split_at(mid); // 借出两个只读切片

    // scope 保证：作用域结束前，里面 spawn 的线程全部 join 完毕。
    // 因此线程可以放心【借用】外面的 data —— 不需要 move、不需要 Arc！
    let (sum_l, sum_r) = thread::scope(|s| {
        let h1 = s.spawn(|| left.iter().sum::<i64>());   // 借用 left
        let h2 = s.spawn(|| right.iter().sum::<i64>());  // 借用 right
        (h1.join().unwrap(), h2.join().unwrap())
    }); // ← 走到这里时两个线程必定已结束，借用安全

    println!("data 还能用: {data:?}，两半的和 = {sum_l} + {sum_r} = {}", sum_l + sum_r);
}
```

为什么它能豁免 `'static`：`scope` 的类型签名向编译器 **证明** 了「线程活不过这个作用域」，借用因此安全——这是「生命周期」机制的一次漂亮应用。  
注意 **tokio 没有对应物**（异步任务的生命周期无法这样静态约束），所以异步世界共享数据老老实实用 `Arc`；  
而纯线程的并行计算里，`scope` 常常让代码干净一大截。

----

# CPU 密集切块并行

> 把作用域线程推广成通用模式——大数组求和，按核数切块，每核一块。这就是 MapReduce 的单机版。

```rust
use std::thread;

fn main() {
    let data: Vec<i64> = (1..=10_000_000).collect();

    // 逻辑核数：并行计算开这么多线程正好吃满 CPU
    let n = thread::available_parallelism().map(|p| p.get()).unwrap_or(4);
    let chunk_size = data.len().div_ceil(n);

    let t = std::time::Instant::now();
    let total: i64 = thread::scope(|s| {
        // chunks()：把切片按块切开借出去；每块给一个线程
        let handles: Vec<_> = data
            .chunks(chunk_size)
            .map(|chunk| s.spawn(move || chunk.iter().sum::<i64>()))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).sum()
    });
    println!("{n} 线程并行求和 = {total}，耗时 {:?}", t.elapsed());
}
```

- 模式总结：**切块（chunks）→ 每块一线程（scope + spawn）→ 收集合并（join + sum）**；
- 对照 Go：`runtime.NumCPU()` + 切片分段 + WaitGroup，形状一致；
- 生产提示：真实项目里这个模式直接用 [`rayon`](https://crates.io/crates/rayon) crate——`data.par_iter().sum()` 一行，  
  内部就是工作窃取线程池。本篇手写一遍是为了知道它替你做了什么；
- **别用异步做这件事**：纯 CPU 计算没有「等待」可让出，塞进 tokio 只会霸占工作线程（[《async 基础》](../async/basics.md) 铁律）。  
  CPU 密集 → 线程/rayon；IO 密集 → 异步。混合场景用 `spawn_blocking` 当桥。

----

# 线程还是异步

> 承上启下：什么时候用线程、什么时候用异步，一张表说清。

| 问题 | 答案 |
| --- | --- |
| 几十个并发的 IO？ | 线程也行、异步也行，怎么顺手怎么来 |
| 几千~几万并发连接？ | 异步（线程成本封顶了并发数） |
| 纯 CPU 并行计算？ | 线程 / rayon（异步帮不上忙，还添乱） |
| 异步代码里遇到同步阻塞的库/重计算？ | `spawn_blocking` 丢给阻塞线程池 |
| 借用栈上数据的并行？ | `thread::scope`（异步没有对应物） |

一句话：**线程和异步不是新旧之争，是两种工况的工具**。Go 用 goroutine 一招打天下（运行时替你兜底）；  
Rust 让你按工况选，换来的是每种工况下的极致成本。

----

# 常见坑清单

> 撞过就长记性，先剧透。

- **忘了 join**：main 返回 = 进程结束 = 所有线程蒸发。集中收集 handles 最后统一 join；
- **在持锁时做慢操作**：[《共享状态：Arc / RwLock》](../async/shared-state.md) 「锁外算好、锁内快进快出」的纪律，  
  线程世界同样适用（后果是别的线程阻塞排队）；
- **线程数远超核数（CPU 密集时）**：100 个计算线程抢 8 个核，上下文切换白白烧 CPU——CPU 密集的线程数 = 核数左右最优；
- **panic 静默丢失**：不 join 的线程 panic 了没人知道。后台常驻线程要自己包 `catch_unwind` 或带日志；
- **死锁**：多把锁的加锁顺序不一致就可能死锁，那是线程时代的经典问题，异步世界也原样成立。

----

# 动手实验

> 第 2 个最直观：亲眼看到多核并行把耗时打下来。

1. **感受「main 退出全体死亡」**：删掉 spawn 例子的 join，多跑几遍看子线程输出被截断在随机位置；
2. **验证真并行**：跑并行求和，把线程数 `n` 改成 1 再跑，对比耗时（多核机器上差距 ≈ 核数倍）；同时开着任务管理器/`htop`，  
   看多线程版把所有核都打满；
3. **重演竞态**：把 `Mutex` 版改成「`AtomicU64` + 拆开的 load/store」，观察结果 < 4000；
4. **体验 scope 的借用检查**：把 `thread::scope` 换回 `thread::spawn`，  
   看编译器怎么拒绝借用；再试着在 scope 里 spawn 一个 **写** data 的线程（`&mut` 借用），  
   编译器同样会拦——体会借用规则在并发下依然生效；
5. **观察线程**：跑并行求和时用 `ps -eLf | grep threads | wc -l`（Linux）或 Process Explorer（Windows）数线程。

----

# 三句话带走

1. **`thread::spawn` + `join` 是 Rust 并发的地基**：真 OS 线程、闭包要 move（`'static` + `Send` 体检与异步同一套）、  
   join 拿返回值/接 panic；main 退出全体蒸发。
2. **共享靠 Arc+Mutex/原子量、通信靠 mpsc** ——和异步版完全同构（那些纪律在这里原样成立）；std 独有的 `thread::scope` 允许借用栈数据，  
   是纯并行计算的利器。
3. **选型**：海量 IO 并发 → 异步；纯 CPU 并行 → 线程/rayon；两界穿梭 → `spawn_blocking`。  
   线程和异步是两种工况的工具，不是新旧之争。

----

# 附：本章生词表

- **`std::thread::spawn(闭包)`** ——创建真 OS 线程执行闭包，立即返回 `JoinHandle<T>`；  
  闭包要求 `'static + Send`（和 `tokio::spawn` 同一套体检）；≈ Go 的 `go`，  
  但有句柄可回收结果。
- **`JoinHandle::join()`** ——阻塞等待线程结束，返回 `Result<T, Box<dyn Any>>`：  
  Ok(返回值) / Err(panic 载荷)；对照 tokio 的 `handle.await`（语义相同，等的方式不同）。
- **`thread::sleep(dur)`** ——同步睡眠：把 **当前线程** 交还调度器指定时长；纯线程里正当（只睡自己），async 里是毒药。
- **`std::sync::mpsc::channel()` / `sync_channel(n)`** ——标准库通道：  
  无界 / 有界带背压；`rx` 实现了 Iterator，`for msg in rx` 循环到所有 tx drop 为止。
- **`thread::scope(|s| { ... })`（作用域线程）** ——保证作用域结束前里面的线程全部结束，  
  因此线程可 **借用** 外部栈数据（豁免 `'static`），不需要 move/Arc；tokio 无对应物。
- **`available_parallelism()`** ——返回逻辑核数（`NonZeroUsize`，用 `.get()` 取值），  
  ≈ Go 的 `runtime.NumCPU()`。
- **`chunks(size)` / `split_at(mid)`** ——切片的分块借用：`chunks` 产出一串 `&[T]` 子切片、  
  `split_at` 一分为二；只读借用可同时借出多份。
- **`div_ceil`** ——向上取整除法（`10.div_ceil(3) == 4`），分块时保证「最后一块不丢」。
- **`rayon`（提及）** ——数据并行库：`data.par_iter().sum()` 一行顶手写的全部；  
  内部是工作窃取线程池，CPU 密集并行的生产标配。
