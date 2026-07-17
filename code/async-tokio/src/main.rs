//! Lesson 02 —— Tokio 运行时：谁在"跑"那些 Future？
//!
//! 上一课说 Future 是惰性的待办卡片。那谁来领卡片、干活、到点叫醒？
//! —— 运行时（runtime）。Go 把运行时内建在语言里（GMP 调度器），
//! Rust 则把它做成可选的库，最主流的就是 Tokio。
//! `#[tokio::main]` 就是帮你启动 Tokio、把 main 交给它驱动。
//!
//! 本课七个场景，对照 Go 的心智映射：
//!   (1) tokio::spawn                ≈ go func(){...}()      （后台任务）
//!   (2) 一批 spawn + 逐个 await     ≈ WaitGroup             （等全部完成）
//!   (3) oneshot 通道                ≈ 一次性的 chan          （送一个结果）
//!   (4) mpsc 多生产者 + drop        ≈ 多 goroutine 写一个 chan + close
//!   (5) interval + select!          ≈ time.NewTicker + select
//!   (6) spawn_blocking              ≈ （Go 不需要，见注释）   （阻塞的正确姿势）
//!   (7) JoinHandle.await            ≈ 用 chan 收 goroutine 结果
//!   (8) CancellationToken           ≈ context.WithCancel      （取消广播）
//!
//! 运行：cargo run -p async-tokio

use std::time::Duration;

use labkit::logln;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tokio::time::{interval, sleep}; 

#[tokio::main]
async fn main() {
    // =====================================================================
    // (1) spawn：真正的「go 一下」—— 把任务丢到后台
    // =====================================================================
    // spawn 和 join! 的区别：join! 在"当前任务里"交替推进；
    // spawn 是把 Future 注册成一个**独立任务**，运行时的线程池随时调度它
    // （多线程运行时下可能真的在另一个 CPU 核上跑 → 真·并行）。
    // spawn 立刻返回一个 JoinHandle（Go 的 go 没有返回值句柄，Rust 直接给你）。
    logln!("(1) spawn 后台任务 -----------------------------------------------");
    let handle = tokio::spawn(async {
        // 这个任务睡 3000ms。注意：它从 spawn 这一刻就开始在后台推进了，
        // 和下面 main 里的所有场景**同时**进行 —— 场景(7)会回来收它。
        logln!("== 后台任务开始睡 3000ms");
        sleep(Duration::from_millis(3000)).await;
        logln!("== 后台任务睡 3000ms 醒了，返回结果"); 
        42
    });
    logln!("后台任务已 spawn（在后台睡 3000ms）。main 不等它，继续往下走\n");

    // =====================================================================
    // (2) 一批任务 + 等全部完成（对照 sync.WaitGroup）
    // =====================================================================
    logln!("(2) 一批任务（WaitGroup 风格）------------------------------------");
    let mut handles = Vec::new();
    for i in 0..3u64 {
        // 注意 async move：把 i 的所有权搬进任务。
        // 为什么必须搬？spawn 的任务要求 'static —— 它可能比当前函数活得久，
        // 不能借用 main 栈上的变量（借用检查器会拦）。move 进去就没有借用问题。
        // 对照 Go：go func(){ fmt.Println(i) }() 直接捕获（Go 有 GC，不需要想这些）。
        handles.push(tokio::spawn(async move {
            logln!("    批量任务 {i} 开始，睡 {}ms", 500 * (i + 1));
            sleep(Duration::from_millis(500 * (i + 1))).await;
            logln!("    批量任务 {i} 完成");
            i * 10 // 任务的返回值
        }));
    }
    // 逐个 .await JoinHandle ≈ wg.Wait()，而且还能拿到每个任务的返回值。
    // .unwrap() 是因为任务可能 panic，JoinHandle.await 返回 Result 让你处理。
    let mut sum = 0;
    for h in handles {
        sum += h.await.unwrap();
    }
    logln!("三个任务的返回值之和: {sum}\n");

    // =====================================================================
    // (3) oneshot：一次性通道，专门用来"送回一个结果"
    // =====================================================================
    // 对照 Go：ch := make(chan string, 1)，发一次、收一次就完事的场景。
    // Rust 专门给这种场景做了 oneshot，类型上就保证只能 send 一次。
    logln!("(3) oneshot 通道 --------------------------------------------------");
    let (otx, orx) = oneshot::channel::<String>();
    tokio::spawn(async move {
        logln!("oneshot 任务开始睡 3000ms");
        sleep(Duration::from_millis(3000)).await;
        // send 不是 async 的（不会满，容量恒为 1），失败说明接收端已丢弃。
        logln!("oneshot 任务睡 3000ms 醒了，发回结果");
        let _ = otx.send("计算完毕".to_string());
    });
    // oneshot 的接收端直接 .await（不是 .recv().await）
    logln!("oneshot 收到: {}\n", orx.await.unwrap());

    // =====================================================================
    // (4) mpsc：多生产者单消费者 + 「通道何时关闭」
    // =====================================================================
    // 对照 Go：ch := make(chan string, 16)，多个 goroutine 往里写。
    // 关键差异：Go 靠手动 close(ch)；Rust 的通道**没有 close 方法**，
    // 关闭的信号是「所有发送端 tx 都被 drop」。
    logln!("(4) mpsc 多生产者 + 背压 --------------------------------------------");
    // ★ 想"看见"容量（背压）的关键：让生产比消费快！
    //   - 生产者：不 sleep，全速连发；
    //   - 消费者：每收一条故意睡 500ms（慢速消费）。
    //   通道很快被塞满，之后生产者的 send().await 会被"顶住"，
    //   只有消费者每取走一条才放行一条。
    //   看"已进入通道"的时间戳：前几条几乎同时，
    //   后面的被迫跟随消费节奏（≈每 500ms 一条）。
    let (tx, mut rx) = mpsc::channel::<String>(3); // 容量 3
    for id in 1..=2 {
        let tx = tx.clone(); // 克隆发送端 → 多生产者（mpsc = multi-producer, single-consumer）
        tokio::spawn(async move {
            for n in 1..=4 {
                logln!("    生产者{id} 想发第 {n} 条…");
                // 通道满时，这个 .await 就地挂起 —— 这就是背压
                tx.send(format!("生产者{id} 的第 {n} 条")).await.unwrap();
                logln!("    生产者{id} 第 {n} 条已进入通道");
            }
            // 任务在这里结束，它手里的 tx 被自动 drop
        });
    }
    // ⚠️ 关键一步：把 main 手里的**原始 tx** 也 drop 掉！
    // 两个生产者拿的是 clone，加上这个原始的一共 3 个发送端；
    // 只有 3 个全消失，rx 才会收到"通道关闭"。漏掉这行 → 下面的循环永远不结束（死等）。
    drop(tx);
    // recv() 返回 Some(消息)；所有发送端都 drop 后返回 None → 循环自然结束。
    // 对照 Go：for msg := range ch { ... }（ch 被 close 后循环结束）。
    while let Some(msg) = rx.recv().await {
        logln!("mpsc 收到: {msg}（消费者故意睡 500ms 制造慢消费）");
        sleep(Duration::from_millis(1000)).await;
    }
    logln!("所有发送端已 drop，通道关闭\n");

    // =====================================================================
    // (5) interval + select!：周期数据源 + 总超时
    // =====================================================================
    // 生产者：每 500ms 发一个数（对照 time.NewTicker）。
    // 消费者：用 select! 同时等「下一条消息」和「总超时」，谁先到就走谁的分支
    // （对照 Go 的 select { case v := <-ch: ... case <-timeout: ... }）。
    logln!("(5) interval + select! --------------------------------------------");
    let (tx, mut rx) = mpsc::channel::<u64>(16);
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(500));
        let mut n = 0u64;
        loop {
            ticker.tick().await; // 第一次 tick 立即返回，之后每 500ms 一次
            n += 1;
            logln!("    ticker 生产者发第 {n} 条");
            if tx.send(n).await.is_err() {
                // send 返回 Err 只有一种可能：接收端已不存在（rx 被 drop / close）。
                // 这就是"消费者反向通知生产者停止"的机制 —— 不需要额外信号。
                logln!("    ticker 生产者：发送失败（接收端已关闭），停止生产");
                break;
            }
            if n >= 8 {
                break; // 发够 8 个，任务结束，tx 被 drop → 通道关闭
            }
        }
    });

    // 超时 Future 要在循环外创建一次、循环内反复引用（&mut）：
    // 如果写在 select! 里面，每轮循环都会新建一个 2000ms 的计时器 = 永远不会超时！
    // Box::pin 是因为 sleep 的 Future 不能直接被反复借用轮询，钉在堆上就可以。
    // （Pin 的原理先不深究，记住这个固定套路即可。）
    let mut deadline = Box::pin(sleep(Duration::from_millis(2000)));
    loop {
        tokio::select! {
            maybe = rx.recv() => {
                match maybe {
                    Some(v) => logln!("select 收到: {v}"),
                    None => { logln!("通道关闭，退出消费循环"); break; }
                }
            }
            _ = &mut deadline => {
                logln!("到达 2000ms 总超时，退出消费循环");
                break;
            }
        }
    }
    // ★ 让生产者也停下来：把接收端 drop 掉！
    // 原理和 drop(tx) 对称：mpsc 的两端互相感知对方是否还活着 ——
    //   - 所有 tx 都 drop → rx.recv() 返回 None（场景(4)用过）；
    //   - rx 被 drop     → tx.send() 返回 Err（生产者据此退出）。
    // 若不写这行，rx 会活到 main 结束，期间生产者的 send 一直成功
    // （消息默默堆进缓冲，没人读），它会傻傻发满 8 条才停。
    // 注意时机：生产者要到**下一次 send** 时才会发现（最多再等一个 tick 周期）。
    drop(rx);
    // 稍等一下，让生产者那句"发送失败，停止生产"的日志打印出来给你看
    sleep(Duration::from_millis(700)).await;
    logln!("");

    // =====================================================================
    // (6) spawn_blocking：非做不可的阻塞，放专用线程池
    // =====================================================================
    // 第 01 课场景(5)说过：阻塞会毁掉并发。那真要做阻塞的事（同步文件 IO、
    // 调用只有同步接口的库、CPU 重计算）怎么办？→ spawn_blocking。
    // 它把闭包丢到 Tokio 专门的「阻塞线程池」执行，不占用异步工作线程。
    // 对照 Go：Go 不需要这个 —— goroutine 阻塞时调度器会自动补线程；
    // Tokio 是协作式调度，需要你**显式**把阻塞代码隔离出去。
    logln!("(6) spawn_blocking ------------------------------------------------");
    let result = tokio::task::spawn_blocking(|| {
        // 注意：这是普通闭包（不是 async），里面可以放心阻塞
        std::thread::sleep(Duration::from_millis(100));
        "阻塞计算的结果"
    })
    .await
    .unwrap();
    logln!("spawn_blocking 返回: {result}\n");

    // =====================================================================
    // (7) 回头收场景(1)的后台任务
    // =====================================================================
    logln!("(7) 等待场景(1) 的后台任务 ------------------------------------------");
    logln!("它从程序一开始就在后台睡 3000ms，和上面所有场景是并发的");
    let result = handle.await.unwrap();
    logln!("后台任务返回: {result}");
    logln!("★ 找找它'醒了'的那条日志：时间戳是开场后 ≈3 秒 —— 早就完成了，");
    logln!("  刚才的 await 只是立刻取走结果。JoinHandle 拿结果不怕'来晚了'\n");

    // =====================================================================
    // (8) CancellationToken：Go context 式的「取消广播」
    // =====================================================================
    // 场景(5)用 drop(rx) 让生产者停下，但有两个局限：
    //   a) 只能通知"通过这条通道发消息"的任务；
    //   b) 对方要等到下一次 send 才发现。
    // CancellationToken（tokio-util crate）解决的就是通用取消：
    //   - token.clone() 分发给任意多个任务（共享同一个取消状态）；
    //   - 任务在 select! 里等 token.cancelled()；
    //   - 任何持有者调用 token.cancel() → 所有任务**立即**被唤醒退出。
    // 对照 Go：这就是 context.WithCancel ——
    //   token.cancelled() ≈ <-ctx.Done()；token.cancel() ≈ cancel()。
    logln!("(8) CancellationToken ----------------------------------------------");
    let token = CancellationToken::new();
    let mut workers = Vec::new();
    for id in 1..=3 {
        let token = token.clone(); // clone 出的 token 共享同一个取消状态
        workers.push(tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(300));
            loop {
                tokio::select! {
                    // 等取消信号：cancel() 一发生，这个分支立即就绪
                    _ = token.cancelled() => {
                        logln!("    工人 {id} 收到取消信号，立即退出");
                        break;
                    }
                    // 平时的活：每 300ms 干一轮
                    _ = ticker.tick() => {
                        logln!("    工人 {id} 干了一轮活");
                    }
                }
            }
        }));
    }

    // 让工人们干 1 秒，然后广播取消
    sleep(Duration::from_millis(1000)).await;
    logln!("main：广播取消！（对比场景(5)：无需等下一个 tick，工人立即退出）");
    token.cancel(); // ≈ Go 的 cancel()

    for w in workers {
        w.await.unwrap();
    }
    logln!("所有工人已退出。生产环境中优雅退出（第 07 课）常配合它使用");
}
