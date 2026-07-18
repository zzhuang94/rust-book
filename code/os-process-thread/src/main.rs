//! 进程 vs 线程：一个进程里开几个线程，看它们"像是同时"在跑
//!
//! 配套文档：docs/concurrency/os-basics.md 「进程是运行实例」「线程共享地址空间」
//! 运行：cargo run -p os-process-thread（先 cd code）
//!
//! 本程序自己就是"一个进程"（一份独立地址空间 + 至少一个线程：main 线程）。
//! 下面在这个进程里再开几个 OS 线程，每个线程打印自己的 thread::current().id()。
//! 主线程最后逐个 join（等它们跑完）。
//!
//! 关键点：这些线程的日志会交错出现（谁先打印不确定），这正是"调度器毫秒级轮转"
//! 造出的"看起来同时"的幻觉——单核机器上它们也会交错，只是轮流执行罢了。

use std::thread;
use std::time::Duration;

use labkit::logln;

fn main() {
    logln!("--- 进程 vs 线程 ---");
    logln!("主线程 id = {:?}（进程里的第一个线程）", thread::current().id());

    const N: usize = 5;
    let mut handles = Vec::with_capacity(N);

    for i in 0..N {
        handles.push(thread::spawn(move || {
            let id = thread::current().id();
            // 故意睡不同长度，让打印顺序更"乱"，凸显调度的不确定性。
            let sleep_ms = 10 * (N - i) as u64;
            logln!("  子线程 #{i} 启动，id = {id:?}，打算睡 {sleep_ms}ms");
            thread::sleep(Duration::from_millis(sleep_ms));
            logln!("  子线程 #{i} 结束，id = {id:?}");
            id
        }));
    }

    logln!("主线程：{N} 个子线程已经 spawn 完，逐个 join 等它们收尾");
    for (i, h) in handles.into_iter().enumerate() {
        let id = h.join().unwrap();
        logln!("主线程：join 到子线程 #{i}（id = {id:?}）");
    }

    logln!("★ 观察上面的时间戳和打印顺序：谁先打印取决于调度器怎么轮转，不是代码顺序决定的");
    logln!("★ 但它们共享同一份进程内存——这就是「同进程线程共享地址空间」的直接体现：");
    logln!("  它们都能调用同一个 logln! 宏、读同一批全局状态，而不需要任何 IPC");
}
