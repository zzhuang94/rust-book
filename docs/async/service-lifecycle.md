# 生产任务生命周期

> 代码：`code/async-service-lifecycle/src/main.rs`　运行：  
> `cargo run -p async-service-lifecycle`

会写 `tokio::spawn`，只代表会启动后台任务；能知道任务何时失败、通知其他任务退出、
等待收尾并在超时后兜底，才算真正管理了服务生命周期。

本课把一个常驻服务缩小成两个任务：`heartbeat` 定时报告存活，`updater` 模拟上游刷新。
刷新任务一秒后故意失败，主任务发现故障后停止整个服务。运行时不需要按 Ctrl-C。
建议先读 [《超时、限流与任务组》](task-control.md)；HTTP 停机入口见
[《中间件与优雅退出》](../http/middleware-shutdown.md)。

----

# spawn 不是管理

下面的代码能启动任务，但不够用于生产：

```rust
tokio::spawn(async {
    background_work().await;
});
```

`spawn` 会立即返回 `JoinHandle`。如果直接丢掉 handle，任务仍会继续运行，
但主流程无法方便地知道它什么时候结束、返回什么错误。

Go 中 `go backgroundWork()` 也有同一个问题：启动容易，错误上报和退出协调要另外设计。

一个常驻服务至少要回答四个问题：

1. 哪些任务是关键任务，提前结束是否意味着进程应该退出？
2. 谁负责接收 Ctrl-C、SIGTERM 或内部故障？
3. 退出信号如何广播给所有任务？
4. 给任务多少收尾时间，超时后怎么办？

----

# JoinSet 统一监管

示例把关键任务放进同一个 `JoinSet`：

```rust
let mut tasks = JoinSet::new();
tasks.spawn(heartbeat(cancel.child_token()));
tasks.spawn(updater(cancel.child_token()));
```

`JoinSet` 是动态任务组。`join_next().await` 不按启动顺序等待，而是哪个任务先结束，
就先拿到哪个结果。这正适合监管长期运行的不同组件。

任务结果看起来有三层：

```rust
Some(Ok(Ok(())))
Some(Ok(Err(e)))
Some(Err(e))
```

从外向内拆：

- `Option`：任务组可能已经空了；
- 外层 `Result`：Tokio 任务本身是否正常完成；panic 或被 abort 会落到 `Err`；
- 内层 `Result`：业务函数返回成功还是业务错误。

所以不能只写一个 `.unwrap()` 把所有失败揉成 panic。生产日志要区分：
“任务没有了”“业务失败”“任务 panic/被取消”，处理策略也可能不同。

> 🔩 底层视角：`JoinHandle` 的错误不是业务函数的错误。前者描述任务执行容器，
> 后者描述任务里面做的事情。两层 `Result` 正是在保存这两个事实。

----

# CancellationToken 广播退出

示例先创建根 token，再给每个任务一个 child token：

```rust
let cancel = CancellationToken::new();
tasks.spawn(heartbeat(cancel.child_token()));
```

主流程调用：

```rust
cancel.cancel();
```

所有子 token 的 `cancelled().await` 都会被唤醒。后台循环通常这样写：

```rust
loop {
    tokio::select! {
        _ = cancel.cancelled() => {
            flush_and_close().await?;
            return Ok(());
        }
        _ = ticker.tick() => do_one_round().await?,
    }
}
```

这叫“协作式取消”：token 不会粗暴杀死任务，只是通知任务“应该停了”。
任务在安全点观察信号，自己释放资源、提交最后一批数据，然后返回。

对应 Go：

```go
select {
case <-ctx.Done():
    return ctx.Err()
case <-ticker.C:
    doOneRound()
}
```

`CancellationToken` 很像 `context.Context` 的取消部分，但它不负责携带请求值和 deadline。
单次操作时限仍应使用 `tokio::time::timeout`。

----

# select 决定谁触发停机

主流程同时等外部信号和内部任务：

```rust
tokio::select! {
    signal = tokio::signal::ctrl_c() => { /* 外部要求停机 */ }
    result = tasks.join_next() => { /* 关键任务提前结束 */ }
}
```

任意一个分支先完成，`select!` 就进入该分支，并取消另一个等待 Future。
注意这里只取消“等待 Ctrl-C”或“等待下一个任务结果”的操作，
并没有自动取消 `JoinSet` 中正在运行的任务，所以后面仍要显式 `cancel.cancel()`。

真实服务的触发源通常包括：

- Ctrl-C 或 SIGTERM；
- HTTP/UDP 监听任务意外退出；
- 配置刷新等关键后台任务失败；
- 管理接口主动要求停机。

要先给任务分级：关键任务失败通常触发全局退出；指标上报等非关键任务失败，
可以记录错误并重启。不要让所有后台任务采用同一种策略。

----

# 五步优雅退出

一个稳妥的退出顺序是：

1. **停止接新活**：关闭 listener 或让 HTTP 服务进入 graceful shutdown；
2. **广播取消**：调用根 `CancellationToken::cancel()`；
3. **任务自行收尾**：循环观察 token，释放资源后返回；
4. **限时等待**：等待 `JoinSet` 清空，但设置总时限；
5. **强制兜底**：超时后 `abort_all()`，避免进程永远退不掉。

示例中的等待代码：

```rust
let drain = async {
    while let Some(result) = tasks.join_next().await {
        // 记录每个任务的最终结果
    }
};

if tokio::time::timeout(Duration::from_secs(2), drain)
    .await
    .is_err()
{
    tasks.abort_all();
}
```

为什么有协作式取消还需要 abort？因为任务可能写错了：阻塞、死循环，或等待一个永远不会完成的操作。
优雅退出必须有期限，否则部署系统发送 SIGTERM 后，新版本会一直等旧进程。

`abort_all()` 也不是万能的。普通异步任务会在下一个可让出点被取消；如果任务正在执行
长时间同步阻塞代码，Tokio 无法立刻抢占它。阻塞工作应放到 `spawn_blocking`，并另外设计停止机制。

----

# 结构化并发

“结构化并发”不是某个语法，而是一条设计原则：子任务的生命应该被父作用域看见和约束。

本例中的结构很清楚：

```text
main
├── heartbeat
└── updater
```

`main` 持有任务组和根取消令牌，因此知道：

- 启动了哪些任务；
- 谁先结束；
- 如何通知全体退出；
- 何时已经全部回收。

如果每个模块随手 spawn 后扔掉 handle，任务的所有权关系就消失了。
测试会偶发残留任务，停机也很难证明已经收干净。

----

# 错误策略先写清

关键任务返回 `Err` 时，可以有三种常见策略：

| 策略 | 适合场景 | 风险 |
| --- | --- | --- |
| 整体退出 | listener、核心状态更新器 | 短暂故障也可能重启进程 |
| 原地重试 | 临时网络错误 | 无上限重试会掩盖永久故障 |
| 保留旧值 | 配置、节点快照等软依赖 | 旧值可能逐渐过期 |

重试应该有退避、抖动和上限；保留旧值应该暴露“最后成功时间”；整体退出应该让部署系统
能够拉起进程。任务监管解决的是“发现并传播”，业务仍要决定“发生后怎么办”。

----

# 常见错误还原

## 主函数直接结束

Tokio runtime 被销毁时，剩余任务会被终止，但这不等于优雅退出。
未刷新的日志、缓存和网络响应可能直接丢失。

## 只等 Ctrl-C

如果只等待 Ctrl-C，关键后台任务早已崩掉，进程仍可能表面存活。
应同时监管内部关键任务。

## 只 cancel 不等待

`cancel()` 只是发通知，不代表任务已经退出。主流程还要 join，确认收尾完成。

## 无限等待收尾

任何 shutdown 都应有总 deadline。单个组件可以有更短的子时限，但不能让总退出时间失控。

## 持锁跨 await

收尾代码如果拿着 `MutexGuard` 再等待网络，很容易拖住其他任务。
先复制必要数据并释放锁，再做异步 IO。

----

# 动手实验

1. 把 `updater` 的一秒失败改成 `Ok(())`，观察“关键任务提前正常结束”；
2. 把 `updater` 的 sleep 改成十秒，运行后按 Ctrl-C，观察两个任务响应取消；
3. 在 `heartbeat` 的取消分支中 sleep 三秒，观察两秒 drain 超时和强制终止；
4. 再增加一个非关键 metrics 任务，尝试设计“失败后重启而不退出进程”的监管循环；
5. 把 `cancel.child_token()` 换成 `cancel.clone()`，阅读文档并比较层级取消能力。

----

# 三句话带走

1. `spawn` 只负责启动；`JoinSet` 才让主流程统一观察、回收和终止动态任务。
2. `CancellationToken` 负责广播协作式取消，任务必须在循环或等待点主动观察它。
3. 生产停机顺序是停止接活、广播取消、限时 drain、超时 abort，且内部故障也要能触发这条链。

----

# 附：本课生词表

- **生命周期（lifecycle）** —— 服务或任务从启动、运行、停止到资源回收的完整过程。
- **监管（supervision）** —— 父任务持续观察子任务状态，并按策略处理退出或失败。
- **`JoinSet<T>`** —— 动态任务组，`join_next` 按完成顺序取得任务结果。
- **`JoinError`** —— Tokio 任务 panic、被取消等执行层错误，不等于业务错误。
- **`CancellationToken`** —— 可克隆、可分层的协作式取消通知工具。
- **协作式取消** —— 任务收到信号后在安全点自行退出，而不是被线程级强制抢占。
- **drain** —— 停止接收新工作后，等待已有工作处理和资源收尾。
- **graceful shutdown** —— 尽量完成必要收尾后再退出的停机过程。
- **fail-fast** —— 关键条件失败时尽早停止整体，避免带病继续运行。
- **结构化并发** —— 让子任务归属于明确父作用域，启动与回收关系可追踪。
