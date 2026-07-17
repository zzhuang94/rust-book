//! Lesson 09 —— 超时、限流、任务组：工程里天天用的三板斧。
//!
//! 对照 Go 的心智映射：
//!   (1) tokio::time::timeout    ≈ context.WithTimeout（单次操作超时）
//!   (2) Semaphore               ≈ 带缓冲 chan 当信号量 / errgroup.SetLimit（限并发）
//!   (3) JoinSet                 ≈ WaitGroup + 结果 chan（动态任务组，完成一个收一个）
//!   (4) try_join!               ≈ errgroup.WithContext（一个失败，全体取消）
//!
//! 运行：cargo run -p async-task-control

use std::sync::Arc;
use std::time::Duration;

use labkit::logln;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::{sleep, timeout};

/// 一个可指定耗时的"慢操作"。开始/完成各打一条日志 ——
/// 被取消的操作永远打不出"完成"，这是观察取消的关键。
async fn slow_op(name: &str, ms: u64) -> String {
    logln!("    slow_op({name}) 开始，需要 {ms}ms");
    sleep(Duration::from_millis(ms)).await;
    logln!("    slow_op({name}) 完成");
    format!("{name} 的结果")
}

/// 模拟会失败的步骤（场景(4)用）。
async fn failing_op(ms: u64) -> Result<String, String> {
    sleep(Duration::from_millis(ms)).await;
    logln!("    failing_op 在 {ms}ms 处失败了");
    Err("数据库挂了".to_string())
}

/// 模拟会成功的步骤（场景(4)用）。
async fn ok_op(name: &str, ms: u64) -> Result<String, String> {
    logln!("    ok_op({name}) 开始，需要 {ms}ms");
    sleep(Duration::from_millis(ms)).await;
    logln!("    ok_op({name}) 完成"); // 被取消时这行不会出现
    Ok(format!("{name} 完成"))
}

#[tokio::main]
async fn main() {
    // =====================================================================
    // (1) timeout：给单次操作加超时
    // =====================================================================
    // 对照 Go：ctx, _ := context.WithTimeout(ctx, 500*time.Millisecond)
    //          然后每个下游调用都要自己检查 ctx。
    // Rust 更直接：timeout(时限, future) 把任意 future 包一层，
    // 返回 Result：Ok(结果) = 按时完成；Err(Elapsed) = 超时。
    // ★ 超时后里面的 future 被 drop —— 回忆第 01 课：
    //   丢弃一个 future 就是取消它，没有半执行的残留。
    logln!("(1) timeout ---------------------------------------------------------");

    // 快活 300ms < 时限 500ms → 按时完成
    match timeout(Duration::from_millis(500), slow_op("快活", 300)).await {
        Ok(v) => logln!("按时完成: {v}"),
        Err(e) => logln!("超时: {e}"),
    }

    // 慢活 900ms > 时限 500ms → 超时，future 被取消
    match timeout(Duration::from_millis(500), slow_op("慢活", 900)).await {
        Ok(v) => logln!("按时完成: {v}"),
        Err(e) => logln!("超时: {e} —— 注意上面没有'slow_op(慢活) 完成'的日志：它被取消了"),
    }
    logln!("");

    // =====================================================================
    // (2) Semaphore：限制并发数
    // =====================================================================
    // 需求：9 个任务要打下游接口，但下游最多扛 3 个并发。
    // 对照 Go 的两种写法：
    //   sem := make(chan struct{}, 3); sem <- struct{}{}; defer func(){ <-sem }()
    //   或 errgroup 的 g.SetLimit(3)。
    // Rust：Semaphore::new(3)，任务先 acquire 拿"许可"，拿不到就异步排队。
    // 许可是一个 guard（RAII，回忆第 03 课）：drop 即归还，忘还这件事不存在。
    logln!("(2) Semaphore 限流（9 个任务，最多 3 并发）---------------------------");
    let sem = Arc::new(Semaphore::new(3)); // 3 张许可证
    let mut handles = Vec::new();
    for id in 1..=9 {
        let sem = Arc::clone(&sem);
        handles.push(tokio::spawn(async move {
            // acquire_owned：从 Arc<Semaphore> 拿一张"自有"许可（可 move 进任务）。
            // 没有空余许可时，这个 .await 就地排队。
            let _permit = sem.acquire_owned().await.unwrap();
            logln!("    任务 {id} 拿到许可，开始干活（500ms）");
            sleep(Duration::from_millis(500)).await;
            logln!("    任务 {id} 干完，归还许可");
            // _permit 在这里 drop → 许可归还 → 排队中的下一个任务被放行
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    logln!("★ 看时间戳：任务 3 个一波，每波间隔 ≈500ms —— 限流生效\n");

    // =====================================================================
    // (3) JoinSet：动态任务组，完成一个收一个
    // =====================================================================
    // 场景(2)用 Vec<JoinHandle> 收任务有两个不便：
    //   a) 只能按 spawn 顺序 await，先完成的也得排队等你来收；
    //   b) Vec 被 drop 时任务**不会**被取消（照跑，变成没人管的孤儿）。
    // JoinSet 解决两者：join_next() 谁先完成先收谁；
    // JoinSet 被 drop 时自动 abort 所有剩余任务（不留孤儿）。
    // 对照 Go：WaitGroup + 结果 chan 的组合，或 errgroup 收集结果。
    logln!("(3) JoinSet（完成一个收一个）----------------------------------------");
    let mut set = JoinSet::new();
    for id in 1..=5u64 {
        set.spawn(async move {
            let ms = 1000 - id * 150; // 故意让后 spawn 的先完成
            sleep(Duration::from_millis(ms)).await;
            (id, ms)
        });
    }
    // join_next：任意一个任务完成就返回 Some(结果)；全部收完返回 None。
    while let Some(res) = set.join_next().await {
        let (id, ms) = res.unwrap();
        logln!("    收到: 任务{id}（耗时 {ms}ms）");
    }
    logln!("★ 收到的顺序是 5→4→3→2→1（按完成先后），不是 spawn 顺序\n");

    // =====================================================================
    // (4) try_join!：并发执行，一个失败全体取消
    // =====================================================================
    // 需求：并发做两件事，任何一件失败就立刻放弃整体（别再浪费时间等另一件）。
    // 对照 Go：errgroup.WithContext —— 一个 goroutine 返回 err，
    //          ctx 被 cancel，其他 goroutine 检查 ctx 后退出。
    // Rust：try_join! 并发推进多个返回 Result 的 future：
    //   - 全部 Ok → 返回 Ok((结果1, 结果2, ...))
    //   - 任何一个 Err → **立即**返回这个 Err，其余 future 被 drop（= 取消）
    logln!("(4) try_join!（一个失败，全体取消）-----------------------------------");
    let t = std::time::Instant::now();
    let result = tokio::try_join!(
        ok_op("加载用户资料", 2000), // 需要 2000ms 才能完成
        failing_op(300),             // 300ms 就失败了
    );
    match result {
        Ok((a, b)) => logln!("全部成功: {a} / {b}"),
        Err(e) => {
            logln!("整体失败: {e}，总耗时 {:?}", t.elapsed());
            logln!("★ 耗时 ≈300ms 而不是 2000ms —— 失败即刻返回；");
            logln!("  且上面没有'ok_op(加载用户资料) 完成'的日志：它被取消了");
        }
    }
    logln!("");
    logln!("对照：tokio::join! 是'等全部完成'（第 01 课），");
    logln!("     try_join! 是'等全部成功或第一个失败' —— 按需选用");
}
