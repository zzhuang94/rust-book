# 谁在驱动 Future

> 代码：`code/async-tokio/src/main.rs`　运行：`cargo run -p async-tokio`（场景 (1)–(8)：  
> spawn / 一批任务 / oneshot / mpsc 多生产者+背压 / interval+select / spawn_blocking / 收后台任务 / CancellationToken 取消广播）

[《async 基础》](basics.md) 留下的问题：谁在不停 `poll` 那些 Future、谁响应 `wake`？答案是 **运行时（runtime）/ 执行器（executor）**。  
Rust 标准库 **不提供** 它，最主流的实现就是 **Tokio**。

----

# Tokio 是异步的 OS

> 接着 [《async 基础》](basics.md) 的比喻讲：`async fn` 写出的是 **菜谱**（Future），`.await` 是 **交单走人**。  
> 但 Rust 语言只定义了菜谱的格式， **没有提供厨房** ——菜谱自己不会变成菜。Tokio 就是那个厨房。

它干三件事，每件都能和操作系统的概念对上：

**1. 调度器——相当于操作系统的排班员。** 操作系统调度 **线程**，Tokio 调度 **任务**（task）。  
`tokio::spawn(async {...})` 就是 async 版的 `thread::spawn`：  
把一份菜谱注册进 Tokio，Tokio 手下养着一小池工人线程（默认 = CPU 核数），谁闲着就抓一个任务推进。  
任务比线程轻得多（几十字节 vs 8MB），开几十万个都没事。

**2. 事件通知——相当于按铃系统。** 任务在 `.await` 处停下时，Tokio 拿着登记表去问操作系统（Linux 上用 epoll）：  
「这 1 万个 socket，哪个来数据了叫我。」操作系统一个电话叫醒 Tokio，Tokio 找到对应任务的 Waker 按铃，  
任务回到就绪队列等工人来推进。这就是「厨房按铃」的具体实现。

**3. 异步版工具——定时器、通道、异步锁。** `sleep().await` 不占线程干等（登记在时间表上）；  
mpsc/oneshot 通道让任务之间传消息；`tokio::sync::Mutex` 拿不到锁就登记排队而不是阻塞线程（[《共享状态：Arc / RwLock》](shared-state.md)）。

对照表（左边是你熟悉的，右边是本课要学的）：

| 概念 | 线程世界 | Tokio 世界 |
| --- | --- | --- |
| 排班员 | OS 内核调度器 | Tokio 调度器（用户态） |
| 干活单位 | 线程 | 任务 task |
| 启动方式 | `thread::spawn` | `tokio::spawn` |
| 等待时 | 线程阻塞，白占资源 | 让出线程给别的任务 |
| 谁来唤醒 | OS 调度器 | epoll/时间表 → Waker 按铃 |
| 切换成本 | 进内核，微秒级 | 用户态函数调用，纳秒级 |

一个关键区别现在能讲清了：**操作系统能强制打断正在跑的线程（抢占），Tokio 不能** ——它只是个库，唯一拿回控制权的时机是任务主动 `.await`。  
所以 Tokio 的调度叫 **协作式**：任务不让位，谁也抢不走它的线程。

----

# 为什么不自带运行时

> Go 自带运行时，Rust 不自带——这是两种设计哲学的分野。

**Go 的哲学**：电池全含。GMP 调度器、netpoller、GC 全部内建；`go` 一下就有并发，零配置；  
代价是这套运行时强制绑定，你没得选、也去不掉。

**Rust 的哲学**：零成本抽象 + 不为没用到的东西付费。语言只提供 `async/await` 语法和 `Future` trait（零成本）；  
用什么驱动 Future 交给库：Tokio、async-std、smol… 按需选；嵌入式/WASM 可以换极小运行时甚至不要运行时。

> **一句话心智模型**：Tokio ≈ 你用 cargo 手动装上去的、可插拔的「Go 运行时」——调度器、定时器、异步网络、通道，一整套设施。

----

# tokio::main 展开

> `#[tokio::main]` 这个宏到底做了什么？把它展开看看。

```rust
#[tokio::main]
async fn main() { ... }
```

展开成：

```rust
fn main() {
    tokio::runtime::Builder::new_multi_thread() // 默认多线程运行时
        .enable_all()        // 打开定时器 + IO 驱动
        .build()
        .unwrap()
        .block_on(async {    // 阻塞当前线程，驱动顶层 Future 到完成
            // 你的 main 函数体
        });
}
```

两个概念：

- **`block_on`**：运行时入口，在当前线程上一直 poll 顶层 Future 直到 Ready。它是同步世界与异步世界的边界，  
  也是唯一合法的「阻塞式等异步」。
- **两种运行时**：

| flavor | 行为 | 适用 |
| --- | --- | --- |
| `new_multi_thread`（默认） | 工作线程池（默认 = CPU 核数）+ 工作窃取 | 常规服务 |
| `new_current_thread` | 只用当前一个线程，任务交替推进 | 测试/明确单线程 |

> 🔬 **底层视角**：工作线程没有任何魔法，就是 `std::thread::spawn` 出来的普通 OS 线程（[《Rust 多线程与并发》](../concurrency/threads.md) 那种），  
> 默认开「逻辑核数」个。特别的只是它们跑的代码——一个死循环：从任务队列取一个状态机、调它的 poll、返回 Pending 就放回等待、  
> 换下一个。所谓「运行时/调度器」，就是这个 **用户态的循环**。内核对「任务」一无所知，它眼里只有几个一直很忙的线程。

对照 Go：多线程运行时的「工作窃取线程池」≈ Go 的 GMP。 **重要差别**：Tokio 是 **协作式** 调度（任务只在 `.await` 处让出），  
Go 带抢占（时间片到了强制切走）；所以 Tokio 里一个不含 `.await` 的 CPU 死循环会 **霸占** 工作线程，  
饿死同线程的其他任务——再次呼应 [《async 基础》](basics.md) 的铁律。

> 🔬 为什么 OS 和 Go 能抢占、Tokio 不能？抢占需要「从外部打断正在跑的代码」：内核靠 **硬件时钟中断** 做到（每毫秒硬件强制把 CPU 交还内核一次）；  
> Go 靠运行时在编译时给代码插的检查点 + 信号。Tokio 只是个库——没有中断可用、也不改你的代码，唯一能拿回控制权的时机就是你主动 `.await` 时 poll 返回。  
> 这不是偷懒，是「零成本」哲学的代价：不插桩、不打断，换来纳秒级切换。

----

# spawn 真正的 go

> `tokio::spawn` 就是 async 版的 `go`：把任务扔到后台独立跑。

```rust
let handle = tokio::spawn(async {
    sleep(Duration::from_millis(3000)).await;
    42
});
// ... main 继续干别的，任务在后台推进 ...
let result = handle.await.unwrap();   // 拿到 42
```

spawn vs join! 的区别：

- `join!`：在 **当前任务里** 交替推进几个 Future；
- `spawn`：把 Future 注册成 **独立任务**，运行时线程池随时调度它（多线程运行时下可能落到别的核 → 真·并行）；  
  spawn **立即返回** `JoinHandle`，任务在后台跑。

对照 Go：`tokio::spawn` ≈ `go`；差别是 Go 的 `go` 不给返回值句柄（要自建 channel），  
Rust 直接给你可 `.await` 的 JoinHandle，还能捕获任务 panic。

场景(7) 的设计就是让你看见这一点：后台任务从程序开始睡 3000ms， **与中间所有场景并发** 推进；等场景(7) 去收它时，  
它早已完成；此刻 `handle.await` **立即返回** ——JoinHandle 拿结果不怕「来晚了」，  
结果会一直等你来取。

----

# 任务要 Send static

> spawn 的任务必须 `Send + 'static`。理解这两个约束，就理解了「无畏并发」。

spawn 的签名要求：

```rust
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where F: Future + Send + 'static, F::Output: Send + 'static
```

- **`'static`**：任务存活时间不确定（可能比当前函数久），所以不能借用当前栈上的局部变量。→ 数据要么 `move` 进去转移所有权，  
  要么用 `Arc` 共享（[《共享状态：Arc / RwLock》](shared-state.md)）。
- **`Send`**：任务可能被工作窃取搬到 **另一个线程** 继续跑，捕获的所有数据必须能安全跨线程。

看到这类报错就知道原因了：

```
error: future cannot be sent between threads safely
   ... the trait `Send` is not implemented for `Rc<...>`
```

（比如任务里用了 `Rc`——非线程安全的引用计数——该换 `Arc`。）

对照 Go：Go 里 `go func(){...}()` 随便捕获，编译器不管线程安全，数据竞争靠 `go run -race` 运行时抓；  
Rust 把检查 **提前到编译期**：`Send`/`Sync` 让「能否跨线程」变成类型问题——这就是「无畏并发」的来源。  
`Send`/`Sync` 到底是什么、编译器怎么判定、为什么 `Rc` 不合格—— [《共享状态：Arc / RwLock》](shared-state.md) 有完整的背景课，  
这里先记住「spawn 会查这两张表」即可。

----

# 阻塞用 spawn_blocking

> 异步任务里绝不能阻塞线程。那真需要阻塞/重计算怎么办？交给 `spawn_blocking` 的专用线程池。

```rust
let result = tokio::task::spawn_blocking(|| {
    // 普通闭包（不是 async），这里可以放心阻塞/死算
    expensive_sync_computation()
}).await.unwrap();
```

- Tokio 维护一个独立、可扩容的 **阻塞线程池** 专跑这种活，与异步工作线程隔离；
- 适用：同步文件 IO、只有同步接口的库、重 CPU 计算。

对照 Go：Go 不需要它——调度器在 goroutine 陷入阻塞系统调用时自动补线程；Rust 需要你 **显式** 划界。这是两种运行时的实感差异。

> 🔬 **底层视角**：阻塞线程池里的也是普通 OS 线程，区别在运行时对它们的 **预期** ——这些线程 **允许** 被内核挂起（阻塞态），  
> 反正没有别的任务排在它们身上；而核心工作线程一旦被内核挂起，它队列里的几百个任务全部陪葬。所以规矩的本质一句话：  
> **会进内核等待的操作，去专门的线程等；核心线程只跑「永不阻塞、Pending 就让位」的状态机。**

----

# 定时器 sleep interval

> 两个异步定时器：一次性 `sleep`、周期性 `interval`。

```rust
// 一次性延时
sleep(Duration::from_millis(50)).await;

// 周期触发（≈ time.NewTicker）
let mut ticker = interval(Duration::from_millis(500));
loop {
    ticker.tick().await;   // 每 500ms 就绪一次
}
```

两个容易踩的细节：

1. **`interval` 的第一次 `tick()` 立即就绪**（不等一个周期）。 [《从零手写 HTTP》](../http/http-from-scratch.md) 的后台任务正利用这点：  
   启动即先更新一次。
2. **补帧行为（MissedTickBehavior）**：某次处理超过一个周期时，默认 Burst（尽快补上落下的 tick）；  
   想「错过就跳过」用 `ticker.set_missed_tick_behavior(Skip)`。生产里这个选择常常很关键。

----

# 任务间通信四件套

> 任务之间传消息的四种通道，各有各的语义。

| Tokio | 语义 | Go 对照 |
| --- | --- | --- |
| `mpsc` | 多生产者单消费者，带容量（背压） | `make(chan T, n)` |
| `oneshot` | 只发一次的一对一，常用于送回结果 | 一次性 `chan T`(cap 1) |
| `broadcast` | 多对多，每个消费者收到每条 | 手动 fan-out |
| `watch` | 只保留最新值 | atomic 存最新值 + 通知 |

本课用了 mpsc（场景(4)）和 oneshot（场景(3)）：

```rust
// mpsc（场景(4) 用容量 3，配合慢消费者演示背压）
let (tx, mut rx) = mpsc::channel::<String>(3);
let tx2 = tx.clone();          // 克隆出多个生产者
tx.send(v).await;              // 满了就地挂起 —— 背压
while let Some(m) = rx.recv().await { ... }  // None = 通道关闭

// oneshot（场景(3)：后台算 3 秒，把结果送回来）
let (otx, orx) = oneshot::channel();
otx.send(result).unwrap();     // 注意：send 不是 async
let v = orx.await.unwrap();    // 注意：接收端直接 .await
```

**通道何时关闭（和 Go 的关键差异）**：Go 手动 `close(ch)`；Rust 的通道 **没有 close 方法** ——关闭信号 = **所有 tx 都被 drop**。  
所以场景(4) 必须 `drop(tx)` 丢掉 main 手里的原始发送端，否则 `while let` 永远等不到 None。

**背压怎么才「看得见」**：容量只在「生产持续快于消费」时才可见——消费无限快时，容量 3 和容量 1000 表现一样。  
场景(4) 让生产者全速发、消费者每条睡 500ms：通道满后，生产者的 `send().await` 被顶住。  
判读要点：**「想发」和「已进入通道」的时间差 = 被背压顶住的时长**。

**反向通知：rx 也能叫停 tx** ——两端互相感知对方死活，机制完全对称：

| 谁消失 | 另一端看到什么 |
| --- | --- |
| 所有 tx 被 drop | `rx.recv()` 返回 None |
| rx 被 drop | `tx.send()` 返回 Err |

消费者超时退出后 `drop(rx)`，生产者 **下一次 send** 时发现 Err 就退出——不需要额外信号。  
对照 Go：channel 只能由发送方 close，接收方想叫停发送方得额外开 done chan 或传 context；  
Rust 靠所有权消失天然完成。局限：只通知「用这条通道」的任务、且要等到下一次 send——通用取消见后面的 CancellationToken。

----

# select 同时等多件

> `select!` 同时等多件事，谁先来处理谁。≈ Go 的 `select`，但有几个 Rust 特有的坑。

```rust
// 场景(5)：生产者每 500ms 发一条（共 8 条），消费端总超时 2000ms
let mut deadline = Box::pin(sleep(Duration::from_millis(2000)));
loop {
    tokio::select! {
        maybe = rx.recv() => match maybe {
            Some(v) => logln!("收到: {v}"),
            None => break,
        },
        _ = &mut deadline => { logln!("总超时"); break; }
    }
}
drop(rx); // 超时退出后 drop(rx)，让生产者也停下（见上文"反向通知"）
```

`select!` 同时 poll 所有分支， **谁先 Ready 执行谁**，其余丢弃。四个 Rust 特有的点：

1. **丢弃 = 取消，且是安全的**（Future 惰性、无栈）。没被选中的分支那一轮只是「没轮到」，Future 还在。
2. **超时 Future 必须循环外创建、循环内 `&mut` 复用**：写在 `select!` 里 = 每轮新建计时器 = 永远不会超时；  
   `Box::pin` 是为了让它能被反复借用轮询（见生词表）。
3. **取消安全性（cancellation safety）**：分支 Future 做到一半被取消，可能留下中间状态。  
   `rx.recv()`、`sleep` 都是取消安全的；自己写复杂 Future 时要留意。
4. **公平性**：多分支同时就绪时随机选一个（防饥饿）；要按书写顺序优先，用 `biased;`。

----

# 取消广播 token

> `drop(rx)` 式的反向通知有局限（只通知一条通道、要等下一次 send）。通用取消用 `CancellationToken`——它就是 Go `context` 的对应物。

```rust
use tokio_util::sync::CancellationToken;

let token = CancellationToken::new();

// 分发给任意多个任务（clone 共享同一个取消状态）
let child = token.clone();
tokio::spawn(async move {
    loop {
        tokio::select! {
            _ = child.cancelled() => break,    // ≈ case <-ctx.Done()
            _ = ticker.tick() => { /* 干活 */ }
        }
    }
});

token.cancel();   // ≈ Go 的 cancel()：所有任务立即被唤醒退出
```

对照表（对 Go 用户几乎零学习成本）：

| Go context | CancellationToken |
| --- | --- |
| `ctx, cancel := context.WithCancel(...)` | `let token = CancellationToken::new();` |
| 把 ctx 传给各 goroutine | `token.clone()` 分发给各任务 |
| `case <-ctx.Done():` | `_ = token.cancelled() =>` |
| `cancel()` | `token.cancel()` |
| 父 cancel 子全 cancel | `token.child_token()` 层级取消 |

三种「叫停」手段的选型：

| 手段 | 生效时机 | 通知范围 | 适用 |
| --- | --- | --- | --- |
| `drop(rx)` | 对方下一次 send | 用这条通道的生产者 | 单通道生产-消费 |
| `rx.close()` | 同上，但缓冲可继续 recv 干完 | 同上 | 优雅收尾 |
| `token.cancel()` | **立即**（cancelled() 分支当场就绪） | 所有 clone 持有者 | 多任务/层级取消 |

[《中间件与优雅退出》](../http/middleware-shutdown.md) 的优雅退出在生产中常配合它：  
收到关闭信号后 `token.cancel()`，让后台任务和 HTTP 服务一起收工。

----

# 动手实验

> 先完整跑一遍、读懂每段时间戳，再动手。

1. **看并行**：改成 `#[tokio::main(flavor = "current_thread")]`，在场景(2) 任务里打印 `std::thread::current().id()`。  
   多线程版能看到不同线程 ID，单线程版全一样。
2. **重演 drop(tx) 死等**：注释掉场景(4) 的 `drop(tx);` 再跑——程序卡在 `while let` 永不结束（3 个发送端只 drop 了 2 个）。
3. **背压对照实验**：把场景(4) 的容量从 3 改成 16（装得下全部 8 条）再跑——所有「已进入通道」瞬间完成、  
   生产者全程无等待；改回 3，重新跟随消费节奏。
4. **霸占线程**：spawn 一个 `loop {}`（无 await）任务再看其他任务。多线程下别的任务或许还能跑；  
   current_thread 下 **全部卡死**。直观感受协作式调度没有抢占。跑完删掉。
5. **补帧行为**：场景(5) 周期改 10ms、每次收到后 sleep 35ms，对比默认 Burst 与 `Skip` 的 tick 节奏。
6. **对比两种取消的时延**：场景(5) 的生产者从 drop(rx) 到真正退出隔了多久（≈ 一个 tick 周期）？  
   场景(8) 的工人从 cancel() 到退出又隔多久（≈ 0ms）？体会「下一次 send 才发现」vs「立即唤醒」。

----

# 三句话带走

1. **Rust 不自带运行时，Tokio 是可插拔的执行器**；`#[tokio::main]` 默认起多线程工作窃取运行时（≈ GMP，  
   但 **协作式、无抢占**）。
2. **spawn 的任务必须 `Send + 'static`** ——Rust 把跨线程数据安全提前到编译期（Go 靠 -race 运行时查）；  
   阻塞/重计算用 `spawn_blocking`。
3. **`spawn / interval / mpsc / oneshot / select! / CancellationToken` 对应 `go / Ticker / chan / 一次性 chan / select / context`**；  
   `select!` 注意 `Box::pin` 复用与取消安全；叫停任务：单通道用 `drop(rx)`，多任务/要立即生效用 token。

----

# 附：本课生词表

> 通用语法见 [《Rust 语法底座》](../start/syntax-primer.md)；async/await/join! 见 [《async 基础》](basics.md) 生词表。

- **`tokio::spawn(future)`** ——把 Future 注册成独立后台任务， **立即返回**；  
  传入 async 块或 async fn 调用结果都行；要求 `Send + 'static`。
- **`JoinHandle<T>`** ——spawn 的返回值，「任务的遥控器」：`.await` = 等结束拿返回值；  
  drop 它任务照跑；`.abort()` 可取消。
- **`handle.await.unwrap()` 的 unwrap** ——await 出来的是 `Result<T, JoinError>`：  
  任务正常结束 `Ok`；任务 **panic** 是 `Err`。对照 Go：goroutine panic 直接炸进程；  
  Tokio 把 panic 包成错误交还给你，进程不死。
- **`mpsc::channel::<String>(16)`** ——建有界通道，返回 `(tx, rx)`；`::<String>` 是 turbofish；  
  16 是容量；tx 可 clone 任意多个，rx 只有一个 ≈ `make(chan string, 16)`。
- **`tx.send(v).await` / `.is_err()`** ——异步发送：满了就地挂起（背压）；返回 `Result`，  
  `Err` = 接收端已 drop。
- **`rx.recv().await`** ——返回 `Option<T>`：`Some(v)` 消息；`None` = 所有 tx 已 drop（关闭）；  
  `while let Some(m) = rx.recv().await {}` ≈ Go 的 `for m := range ch {}`。
- **`drop(tx)`** ——立即结束一个值的所有权（提前析构）；对发送端 drop = 「我这路不再发了」；  
  原始 tx 不 drop，rx 永远等不到关闭。
- **`oneshot::channel::<T>()`** ——一次性通道，专用于「送回一个结果」；与 mpsc 两点不同：  
  `otx.send(v)` **不是** async；接收端 **直接** `orx.await`。
- **`interval(d)` / `ticker.tick().await`** ——周期定时器 ≈ `time.NewTicker`；  
  第一次 tick **立即** 返回；ticker 须声明 `mut`。
- **`tokio::select! { 绑定 = future => 代码, ... }`** ——宏：同时等多个 Future，  
  谁先完成执行谁的分支，其余放弃；分支语法 ≈ Go 的 `case v := <-ch:`。
- **`Box::pin(...)` + `&mut deadline`** ——固定套路：`Box` 放堆上、`pin` 钉住不许挪（Future 可能自引用），  
  钉好后用 `&mut` 传给 `select!` 让同一个计时器跨轮存活。口诀：「`select!` 循环里要复用的 Future → 循环外 `Box::pin`，  
  循环内 `&mut`」。
- **`tokio::task::spawn_blocking(|| {...})`** ——把 **普通闭包**（非 async）丢到专用阻塞线程池，  
  返回 JoinHandle；适用同步 IO、同步库、重计算；Go 无对应物。
- **`rx.close()`** ——接收端主动关闭通道但 rx 还在：之后新 send 全失败，缓冲里已有的还能 recv 干完；  
  适合「不收新活，但把手头处理完」的优雅收尾。
- **`tokio_util::sync::CancellationToken`** ——`tokio-util` crate 的取消广播原语；  
  `clone` 分发副本共享取消状态；`cancelled()` 返回「被取消时完成」的 Future 放进 `select!`；  
  `cancel()` 广播取消所有等待者 **立即** 唤醒 ≈ Go 的 `cancel()`；`child_token()` 层级取消。
