# 三类控制手段

> 代码：`code/async-task-control/src/main.rs`　运行：`cargo run -p async-task-control`（场景 (1)–(4)：  
> timeout / Semaphore 限流 / JoinSet / try_join!）

这一课补的是 **工程里出现频率最高** 的三类控制手段。在 Go 里它们分别对应 `context.WithTimeout`、  
信号量/`errgroup.SetLimit`、`WaitGroup`/`errgroup`——你天天在写的东西，  
这里是 Rust 版。

----

# timeout 单次超时

> 给单次操作加超时：`timeout(时限, future)` 一行包一层。

```rust
use tokio::time::timeout;

match timeout(Duration::from_millis(500), slow_op("慢活", 900)).await {
    Ok(v) => logln!("按时完成: {v}"),
    Err(e) => logln!("超时: {e}"),   // e 是 Elapsed 类型
}
```

要点：

- `timeout(时限, future)` 把 **任意** future 包一层，返回 `Result<T, Elapsed>`；
- 按时完成 → `Ok(结果)`；到点没完成 → `Err(Elapsed)`；
- **超时后里面的 future 被 drop = 被取消**（回忆 [《async 基础》](basics.md)：  
  丢弃即取消）。代码里被超时的 `slow_op` 永远打不出「完成」日志——这是观察取消的关键证据。

对照 Go：

```go
ctx, cancel := context.WithTimeout(ctx, 500*time.Millisecond)
defer cancel()
result, err := doSomething(ctx)   // 下游得自己层层检查 ctx
```

差异：Go 的超时要靠 **被调用方配合**（层层传 ctx、层层检查）；Rust 的 timeout 是 **调用方单方面** 说了算——不管里面的 future 配不配合，  
到点直接 drop 掉它。 [《Tokio 运行时》](tokio.md) 场景(5) 那种 `Box::pin(sleep) + select!` 是「循环内多轮共用一个总超时」的写法；  
**单次调用直接 `timeout()` 一行**，别手搓。

----

# Semaphore 限并发

> 限制并发数：9 个任务要打下游接口，但下游最多扛 3 个并发。

```rust
let sem = Arc::new(Semaphore::new(3));   // 3 张许可证
for id in 1..=9 {
    let sem = Arc::clone(&sem);
    tokio::spawn(async move {
        let _permit = sem.acquire_owned().await.unwrap(); // 没许可就排队
        do_work().await;
        // _permit 在这里 drop → 自动归还许可
    });
}
```

要点：

- `Semaphore::new(n)`：n 张许可；`acquire` 拿一张，没有空余就 **异步排队**（不阻塞线程）；
- 许可是一个 **guard（RAII， [《共享状态：Arc / RwLock》](shared-state.md) 的老朋友）**：drop 即归还——「忘了还许可」这件事不存在；
- `acquire_owned()`（配 `Arc<Semaphore>`）拿到的许可可以 move 进 spawn 的任务；  
  普通 `acquire()` 拿到的是借用，跨任务用不了；
- 运行代码看时间戳：任务 **3 个一波**，每波间隔 ≈500ms。

对照 Go 的两种惯用写法：

```go
// 写法一：带缓冲 chan 当信号量
sem := make(chan struct{}, 3)
sem <- struct{}{}          // acquire
defer func() { <-sem }()   // release（忘写 defer 就泄漏了）

// 写法二：errgroup
g.SetLimit(3)
```

Rust 版的优势：归还靠 RAII 自动，不依赖你记得写 defer。

----

# JoinSet 动态任务组

> 动态任务组，完成一个收一个。 [《Tokio 运行时》](tokio.md) 场景(2) 用 `Vec<JoinHandle>` 收任务有两个不便：  
> 只能按 spawn 顺序 await；Vec 被 drop 时任务不会被取消（变成孤儿）。`JoinSet` 解决两者。

```rust
let mut set = JoinSet::new();
for id in 1..=5 {
    set.spawn(async move { /* ... */ (id, result) });
}
while let Some(res) = set.join_next().await {
    let (id, result) = res.unwrap();   // 谁先完成先收谁
}
```

要点：

- `set.spawn(fut)`：往任务组里加一个任务（随时可加，动态的）；
- `join_next().await`：**任意一个** 任务完成就返回 `Some(结果)`；全部收完返回 `None`；
- 代码里故意让后 spawn 的先完成，收到的顺序是 5→4→3→2→1——按完成先后，不是 spawn 顺序；
- **JoinSet 被 drop 时自动 abort 所有剩余任务**（不留孤儿），还有 `set.abort_all()` 手动全取消——这是 `Vec<JoinHandle>` 没有的。

对照 Go：≈ `WaitGroup` + 结果 chan 的组合（每个 goroutine 完成时往 chan 发结果，  
主协程 range 收）；或 errgroup 收集。JoinSet 把这套模板收敛成一个类型。

----

# try_join 一败全取消

> 并发执行，任何一件失败就立刻放弃整体（别再浪费时间等另一件）。

```rust
let result = tokio::try_join!(
    ok_op("加载用户资料", 2000),  // 要 2000ms
    failing_op(300),              // 300ms 就失败
);
match result {
    Ok((a, b)) => { /* 全部成功 */ }
    Err(e) => { /* 300ms 时就走到这里 */ }
}
```

语义：并发推进多个 **返回 Result** 的 future；全部 Ok → `Ok((r1, r2, ...))`；  
任何一个 Err → **立即** 返回该 Err，其余 future 被 drop（= 取消）。代码实证：总耗时 ≈300ms 而非 2000ms，  
且「加载用户资料 完成」的日志不会出现（它被取消了）。

三兄弟对照：

| 宏 | 语义 | Go 对照 |
| --- | --- | --- |
| `join!` | 等 **全部完成**（不管成败） | WaitGroup |
| `try_join!` | 全部成功，或 **第一个失败即返回** | errgroup.WithContext |
| `select!` | 等 **第一个完成**，其余取消 | select |

> 补充：`join!`/`try_join!` 在 **当前任务内** 并发（不开新任务）。如果想要「spawn 出去的任务组 + 一个失败全取消」，  
> 用 JoinSet + 遇错 `abort_all()`，或配合 [《Tokio 运行时》](tokio.md) 的 CancellationToken。

----

# 动手实验

1. **看取消的证据**：把场景(1) 慢活的耗时从 900ms 改成 400ms（小于时限 500ms），「完成」日志重新出现——对比体会「超时 = future 被 drop」；
2. **调限流参数**：场景(2) 把许可从 3 改成 1（串行）和 9（不限），对比总耗时（≈4.5s / ≈0.5s）；
3. **孤儿任务实验**：把场景(3) 的 `while let` 收结果循环删掉，在 JoinSet drop 前加一句 sleep(2s)——任务还是会被收割吗？  
   再换成 `Vec<JoinHandle>` 版本对比（Vec drop 后任务照跑）；
4. **fail-fast 对比**：把场景(4) 的 `try_join!` 改成 `join!`（去掉错误短路），总耗时变回 ≈2000ms。

----

# 三句话带走

1. **单次操作超时用 `timeout()` 一行**，不用手搓 select + sleep；超时 = 内部 future 被 drop = 取消，  
   干净利落。
2. **限并发用 `Semaphore`**：`acquire_owned` 拿许可、guard drop 自动归还——Go 的「缓冲 chan 当信号量」+defer，  
   Rust 用 RAII 免记忆。
3. **动态任务组用 `JoinSet`**（完成一个收一个、drop 自动收割）； **「一败全取消」用 `try_join!`**（≈ errgroup.WithContext）。

----

# 附：本课生词表

> 通用语法见 [《Rust 语法底座》](../start/syntax-primer.md)；spawn/JoinHandle 见 [《Tokio 运行时》](tokio.md) 生词表。

- **`tokio::time::timeout(dur, fut)`** ——给任意 future 包一层时限，返回 `Result<T, Elapsed>`；  
  `Elapsed` 是超时错误类型；到点后内部 future 被 drop——单方面取消，无需对方配合。
- **`Semaphore::new(n)` / `acquire_owned()`** ——信号量：n 张许可，acquire 拿一张（没有就异步排队）；  
  `acquire_owned()` 要在 `Arc<Semaphore>` 上调用，返回 `OwnedSemaphorePermit`（可 move 进任务）；  
  普通 `acquire()` 返回借用型许可，不能跨 spawn 边界。
- **`OwnedSemaphorePermit`（那个 `_permit`）** ——RAII guard：活着 = 占着一张许可，  
  drop = 归还；变量名以 `_` 开头（`_permit`）：告诉编译器「我不使用它，但 **要它活到作用域结束**」——写成纯 `_` 会立即 drop，  
  限流就失效了！这是一个经典陷阱。
- **`JoinSet<T>`** ——动态任务组：`set.spawn(fut)` 加任务、`join_next().await` 收「下一个完成的」；  
  全收完返回 None；drop JoinSet = abort 所有剩余任务；`abort_all()` 手动全取消。
- **`tokio::try_join!(f1, f2, ...)`** ——并发推进多个返回 Result 的 future；  
  全 Ok → `Ok(元组)`；首个 Err → 立即返回、其余 drop；与 `join!`（等全部）、`select!`（等第一个）构成三兄弟。
