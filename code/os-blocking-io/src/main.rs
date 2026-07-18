//! .await 不阻塞线程 vs thread::sleep 会阻塞线程
//!
//! 配套文档：docs/concurrency/os-basics.md 「阻塞的确切含义」「IO 模型三代」
//! 运行：cargo run -p os-blocking-io（先 cd code）
//!
//! `tokio::time::sleep(..).await` 底下没有真的让任何 OS 线程"阻塞"：它把任务注册进
//! 运行时的定时器，任务本身让出（poll 返回 Pending），线程立刻被派去干别的活，
//! 时间到了运行时再用 Waker 把任务重新排进队列。
//! 而 `std::thread::sleep` 是真的调用系统调用让**当前线程**进入"阻塞"态——
//! 如果这个线程是 Tokio 的工作线程，它身上排队的其它任务会跟着一起被冻住。
//!
//! 下面用单线程 Tokio 运行时（故意只给 1 个工作线程，效果最明显）做对比：
//!   A) 任务 1 用 `tokio::time::sleep`（.await）睡 300ms，同时任务 2 每 50ms 打一次心跳；
//!   B) 换成任务 1 用 `std::thread::sleep`（同步阻塞）睡 300ms，观察任务 2 的心跳是否还能打出来。

use std::time::{Duration, Instant};

use labkit::logln;

async fn heartbeat_task(label: &str, total: Duration) {
    let start = Instant::now();
    let mut tick = 0;
    while start.elapsed() < total {
        tokio::time::sleep(Duration::from_millis(50)).await;
        tick += 1;
        logln!("  [{label}] 心跳 #{tick}（已过 {:?}）", start.elapsed());
    }
}

#[tokio::main(flavor = "current_thread")] // 故意只用 1 个线程，让"阻塞会连累谁"这件事无处可藏
async fn main() {
    logln!("--- (A) 用 tokio::time::sleep（.await）：不阻塞线程 ---");
    logln!("单线程运行时：任务 1 sleep 300ms，同时任务 2 每 50ms 打心跳");
    let t = Instant::now();
    tokio::join!(
        async {
            logln!("  [任务1] 开始 tokio::time::sleep(300ms)");
            tokio::time::sleep(Duration::from_millis(300)).await;
            logln!("  [任务1] sleep 结束");
        },
        heartbeat_task("任务2", Duration::from_millis(300)),
    );
    logln!(
        "(A) 结束，耗时 {:?} —— 心跳能正常打出来，说明 sleep 期间线程没被真正占住\n",
        t.elapsed()
    );

    logln!("--- (B) 用 std::thread::sleep（同步阻塞）：会阻塞整个工作线程 ---");
    logln!("同样单线程运行时：任务 1 换成同步 sleep 300ms，同时任务 2 还是每 50ms 打心跳");
    let t = Instant::now();
    tokio::join!(
        async {
            logln!("  [任务1] 开始 std::thread::sleep(300ms)（危险动作：这是同步阻塞）");
            // 故意在 async 块里犯这个"async 铁律"里说的错误：调用会阻塞线程的同步 API。
            std::thread::sleep(Duration::from_millis(300));
            logln!("  [任务1] std::thread::sleep 结束");
        },
        heartbeat_task("任务2", Duration::from_millis(300)),
    );
    logln!("(B) 结束，耗时 {:?}", t.elapsed());
    logln!("★ 对比 (A) 和 (B) 的日志：(B) 里任务2 的心跳会被「任务1」死死卡住，");
    logln!("  只有任务1 的 std::thread::sleep 醒了之后，心跳才补打——因为单线程运行时里，");
    logln!("  这一个线程被同步 sleep 真正阻塞住了，其它任务全部停摆，不管排了多少个。");
}
