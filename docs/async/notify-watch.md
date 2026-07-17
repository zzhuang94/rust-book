# 四种同步原语

> 代码：`code/async-notify-watch/src/main.rs`　运行：`cargo run -p async-notify-watch`（场景 (1)–(4)：  
> watch 热更新 / broadcast 扇出 / Notify 信号 / OnceCell 懒初始化）

[《Tokio 运行时》](tokio.md) 学了 mpsc 和 oneshot。这一课补齐 tokio 通道/同步原语的另外四件，  
它们覆盖的都是工程里的真实场景：**配置热更新、事件广播、纯信号、全局资源懒初始化**。

----

# watch 只留最新值

> `watch` 是「只保留最新值」的通道——配置热更新的标准答案。

它和 mpsc 的本质区别：

| | mpsc | watch |
| --- | --- | --- |
| 心智模型 | **队列**：每条消息都要被消费 | **一格储物柜**：新值覆盖旧值 |
| 读者错过中间值 | 不会（都排着队） | 会， **且这正是想要的** |
| 多读者 | 不行（single-consumer） | 可以（Receiver 随便 clone） |
| 典型场景 | 任务队列、数据流 | 配置热更新、状态发布 |

配置热更新要的语义恰恰是「错过的中间版本根本不重要，给我最新的就行」——watch 天生就是它。

```rust
let (cfg_tx, cfg_rx) = watch::channel(0u64);  // 带初始值

// 读者（可 clone 出任意多个）
let mut rx = cfg_rx.clone();
while rx.changed().await.is_ok() {   // 异步等"值变了"；写端 drop → Err
    let v = *rx.borrow();            // 读当前最新值
    // ...
}

// 写者
cfg_tx.send(new_value).unwrap();     // 覆盖储物柜
```

要点：

- `changed().await`：挂起直到值发生变化；写端 drop 后返回 Err → 读者循环自然退出；
- `borrow()`：借用当前值（读完尽快放， **别跨 await 持有** ——它内部是个读锁， [《共享状态：Arc / RwLock》](shared-state.md) 的铁律同样适用）；
- 代码实证：写者每 400ms 发一版、读者每轮 900ms，读者看到的版本 **跳着走**（如 1→3→5）——中间版本被覆盖了。

「定期更新 + 高并发读」至此有了 **三种解法**：

| 解法 | 读的代价 | 独有能力 | 出处 |
| --- | --- | --- | --- |
| `Arc<RwLock<T>>` | 读锁 | 可局部修改 | [《从零手写 HTTP》](../http/http-from-scratch.md) |
| `ArcSwap<T>` | 无锁取指针 | 读极致性能 | [《ArcSwap 无锁读》](../http/arcswap.md) |
| `watch::channel` | 读锁（borrow） | **值变了能异步等到通知** | 本课 |

只是读最新值 → ArcSwap 最快；读者需要「值一变就立刻做点什么」（重建缓存、重连、刷新路由）→ watch 是唯一带通知语义的。  
对照 Go：没有现成原语——通常 `atomic.Pointer` 存配置 + 另开 chan 做通知，两件事自己拼；  
watch 一个类型全包。

----

# broadcast 人人都收到

> `broadcast`：一条消息，每个订阅者都收到（真广播）。

```rust
let (event_tx, _) = broadcast::channel::<String>(8);

// 每个订阅者一个独立接收端
let mut rx = event_tx.subscribe();
tokio::spawn(async move {
    while let Ok(msg) = rx.recv().await {   // Err(Closed) = 发送端全 drop
        // 每个订阅者都会收到每一条
    }
});

event_tx.send("事件".into()).unwrap();
```

和 mpsc 的语义对比（最容易混）：**mpsc** 一条消息只被 **一个** 消费者拿到（大家「抢活干」）； **broadcast** 每个订阅者收到 **每一条**（真广播）。

要点：

- `subscribe()` 随时可调，订阅从那一刻起的消息；
- 容量（这里 8）满时， **最慢的订阅者** 会丢最旧的消息，下次 recv 得到 `Err(Lagged(n))` 告诉它丢了几条——这是刻意设计：  
  不让慢订阅者拖垮全局（背压的反面取舍）；
- 发送端全部 drop → 订阅者 recv 得 `Err(Closed)` → 退出。

对照 Go：得自己维护 `[]chan` 逐个发，还要处理慢订阅者阻塞全局的问题（要么丢、要么开大缓冲、要么 select default）——broadcast 把这些取舍打包好了。

----

# Notify 纯信号

> `Notify`：不带数据的「叫醒」信号。

```rust
let notify = Arc::new(Notify::new());

// 等待方
notify.notified().await;   // 挂起，直到有人叫

// 通知方
notify.notify_one();       // 唤醒一个等待者
notify.notify_waiters();   // 或：唤醒当前所有等待者
```

- 「有事发生了，醒一醒」——不需要传值时，用 Notify 比开一条 channel 更轻、意图更清晰；
- 对照 Go：`sync.Cond`，或惯用的 `make(chan struct{})` + `close(ch)` 当一次性信号；
- 一个细节：`notify_one()` 在没人等的时候会存下「一次通知额度」，下一个来 `notified()` 的人直接通过（不会丢信号）；  
  `notify_waiters()` 则只叫醒 **当时** 在等的人，不存额度。

----

# OnceCell 懒初始化

> `OnceCell`：能 await 的懒初始化。第一次用到时才初始化，多任务同时抢只执行一次，其余等结果。

```rust
static CONN: OnceCell<String> = OnceCell::const_new();

async fn get_conn() -> &'static String {
    CONN.get_or_init(|| async {
        // 这段初始化代码全程序只会执行一次
        expensive_connect().await
    })
    .await
}
```

要点：

- `OnceCell::const_new()` 是 const 函数 → 能用在 `static`（全局变量）上；
- `get_or_init(闭包)`：没初始化 → 执行闭包（**闭包返回 async 块**，所以初始化过程可以 await）；已初始化 → 直接返回引用；
- 并发安全：三个任务同时调用，只有一个执行初始化，其余两个 **异步等待** 它完成后拿同一个结果——代码里「正在建立连接」只打印一次；
- 返回 `&'static String`：全局单例的引用，随处可用。

对照 Go：≈ `sync.Once`，但 Once 的 `f func()` **不能 await**；Go 里异步初始化单例要 Once + chan 自己拼。  
也 ≈ `std::sync::LazyLock`，但 LazyLock 的初始化是同步的—— **初始化过程要 await 就只能用 OnceCell**。

----

# 四件套选型速查

| 我想要… | 用 |
| --- | --- |
| 发布「最新状态」，读者可等变更通知 | `watch` |
| 一条事件让所有订阅者都知道 | `broadcast` |
| 不带数据的「叫醒」信号 | `Notify` |
| 全局资源第一次用时才（异步）初始化 | `OnceCell` |
| 任务队列（一条只给一个人） | `mpsc`（[《Tokio 运行时》](tokio.md)） |
| 一次性送回一个结果 | `oneshot`（[《Tokio 运行时》](tokio.md)） |

----

# 动手实验

1. **观察「跳版本」**：把场景(1) 读者的 sleep 从 900ms 改成 100ms（变成快读者）——现在 5 个版本一个不落全看到；  
   改回 900ms，又开始跳。体会「watch 不保证每个值都被看到」；
2. **制造 Lagged**：场景(2) 把 broadcast 容量从 8 改成 2，给订阅者的循环里加 `sleep(500ms)`，  
   发 6 条事件——观察慢订阅者收到 `Err(Lagged)`（需要把 `while let Ok` 改成 match 才能看到错误分支）；
3. **验证通知额度**：场景(3) 把 `notify_one()` 挪到 spawn 等待者 **之前** 执行——等待者还能被唤醒吗？  
   （能：notify_one 存了一次额度。）换成 `notify_waiters()` 再试（不能：没人等时白喊）；
4. **感受 OnceCell 的并发合并**：场景(4) 把任务数从 3 改成 10，确认「正在建立连接」依然只打印一次。

----

# 三句话带走

1. **watch = 「最新值 + 变更通知」**，配置热更新的标准答案，也是本项目「定期写+并发读」的第三种解法（唯一带通知的）。
2. **broadcast 每个订阅者收到每一条**（mpsc 是抢活干）；慢订阅者会 Lagged 丢旧消息——防拖垮全局的刻意取舍。
3. **Notify 是不带数据的信号**（≈ sync.Cond）； **OnceCell 是能 await 的 sync.Once**，  
   全局资源懒初始化用它。

----

# 附：本课生词表

> 通用语法见 [《Rust 语法底座》](../start/syntax-primer.md)；mpsc/oneshot 见 [《Tokio 运行时》](tokio.md) 生词表。

- **`watch::channel(初始值)`** ——创建「储物柜」通道， **必须给初始值**；返回 `(Sender, Receiver)`；  
  Receiver 可 clone（多读者）。
- **`rx.changed().await`** ——挂起直到值被 send 过（发生变化）；返回 `Result<(), RecvError>`：  
  Err = 发送端已 drop；惯用 `while rx.changed().await.is_ok() { ... }`。
- **`rx.borrow()`** ——借用当前最新值（返回一个守卫，内部是读锁）；读完尽快放、 **别跨 await 持有**；  
  想拿走值就 `rx.borrow().clone()`；姐妹方法 `borrow_and_update()` 借用同时标记「这版我看过了」。
- **`broadcast::channel(容量)` / `subscribe()`** ——广播通道：每个 `subscribe()` 得到独立接收端，  
  收到订阅之后的每一条消息；返回元组里的 Receiver 常不用（`(event_tx, _)`）。
- **`Err(Lagged(n))`（broadcast 特有）** ——慢订阅者的缓冲被覆盖时，recv 返回它：  
  告诉你丢了 n 条；处理方式通常是记条日志、继续收；设计哲学：广播不为慢订阅者做背压，宁可丢消息不拖全局。
- **`Notify` / `notified()` / `notify_one()` / `notify_waiters()`** ——纯信号原语：  
  `notified().await` 等、`notify_one()` 叫一个、`notify_waiters()` 叫全部；  
  `notify_one` 无人等时存一次「额度」（信号不丢）；`notify_waiters` 不存额度。
- **`OnceCell<T>` / `const_new()` / `get_or_init(...)`** ——只写一次的异步单元格；  
  `const_new()` 是 const fn → 可用于 `static`；`get_or_init(|| async { ... }).await` 闭包返回 async 块，  
  初始化可 await；并发调用只执行一次，其余等待复用；对照 `sync.Once`（不能 await）、`std::sync::LazyLock`（初始化必须同步）。
- **`static CONN: OnceCell<String>`（函数里的 static）** ——`static` 可以声明在函数体内：  
  作用域受限但生命周期全局；配 OnceCell/LazyLock 是声明「局部可见的全局单例」的惯用法。
