//! 调度直觉：忙等线程 vs 睡眠线程，谁在"占着"核心
//!
//! 配套文档：docs/concurrency/os-basics.md 「调度决定跑谁」「阻塞的确切含义」「时间片与抢占」
//! 运行：cargo run -p os-scheduling（先 cd code），全程控制在几秒内。
//!
//! 线程有三种状态：运行（占着核心）、就绪（排队等核心）、阻塞（在等事件，不占核心也不排队）。
//! 下面开两类线程：
//!   - "忙等线程"：死循环空转，一直处于"运行"或"就绪"状态，疯狂消耗 CPU；
//!   - "睡眠线程"：thread::sleep，进入"阻塞"状态，完全不占核心。
//! 观察：睡眠线程几乎不影响忙等线程的吞吐；而增加忙等线程数量，
//! 会让它们互相抢占（核心数有限时，多个"就绪"线程要排队分时间片）。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use labkit::logln;

fn main() {
    let n_cores = thread::available_parallelism().map(|p| p.get()).unwrap_or(4);
    logln!("--- 调度直觉：忙等线程 vs 睡眠线程 ---");
    logln!("可用逻辑核数 = {n_cores}");

    // 忙等线程数故意设成"核数的两倍"，制造"线程比核心多，得排队"的场景。
    let busy_threads = n_cores * 2;
    let run_for = Duration::from_millis(800);
    let counter = Arc::new(AtomicU64::new(0));

    logln!("开 {busy_threads} 个忙等线程（是核数的 2 倍），空转 {run_for:?}，边转边累加计数器");
    let t = Instant::now();
    let deadline = t + run_for;
    let handles: Vec<_> = (0..busy_threads)
        .map(|i| {
            let counter = Arc::clone(&counter);
            thread::spawn(move || {
                let mut local = 0u64;
                // 忙等：不调用任何会"阻塞"的 API，一直处于运行/就绪状态，疯狂抢核心。
                while Instant::now() < deadline {
                    local = local.wrapping_add(1);
                }
                counter.fetch_add(local, Ordering::Relaxed);
                if i == 0 {
                    logln!("  忙等线程 #0 结束（其它线程结束顺序不重要，省略打印）");
                }
            })
        })
        .collect();

    // 同时再开一个"睡眠线程"：它大部分时间是"阻塞"态，几乎不占核心，
    // 因此几乎不影响上面忙等线程的总吞吐——这是"阻塞不占核心"的直接体现。
    let sleeper = thread::spawn(move || {
        logln!("  睡眠线程：开始睡 {run_for:?}（这段时间它处于阻塞态，不占任何核心）");
        thread::sleep(run_for);
        logln!("  睡眠线程：睡醒了");
    });

    for h in handles {
        h.join().unwrap();
    }
    sleeper.join().unwrap();

    let total = counter.load(Ordering::Relaxed);
    let elapsed = t.elapsed();
    logln!("全部结束，实际耗时 {elapsed:?}，{busy_threads} 个忙等线程总共完成 {total} 次空转累加");
    logln!("★ 直觉：");
    logln!("  1) 睡眠线程几乎没拖慢忙等线程——阻塞态的线程不占核心，调度器压根不用管它；");
    logln!("  2) 忙等线程数（{busy_threads}）超过核数（{n_cores}）时，它们要轮流分时间片，");
    logln!("     单个线程实际拿到的 CPU 时间比核数充足时更少——这就是「抢占式调度」在起作用");
}
