//! 空循环 vs 带锁竞争的临界区：耗时差几个数量级
//!
//! 配套文档：docs/concurrency/os-basics.md 「上下文切换的成本」
//! 运行：cargo run -p os-perf-cost（先 cd code，建议 --release 观察更明显）
//!
//! 单线程空转一个循环，几乎是 CPU 能做的最便宜的事——没有系统调用、没有同步、
//! 没有跨核通信。一旦引入"多个线程争用同一把锁"，即使临界区本身极短（就一次加法），
//! 每次拿锁都可能牵扯到：缓存行在核心间同步、锁内部的原子指令、竞争激烈时甚至
//! 让线程真正休眠等待（触发调度器/上下文切换）。这里做一个小规模的粗略对比，
//! 感受一下"量级差异"，不是严格 benchmark。

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use labkit::logln;

const ROUNDS_PER_THREAD: u64 = 500_000;
const THREADS: usize = 4;

/// 基线：单线程空转，纯加法，没有任何同步开销。
fn bench_empty_loop() -> std::time::Duration {
    let t = Instant::now();
    let mut acc: u64 = 0;
    for i in 0..(ROUNDS_PER_THREAD * THREADS as u64) {
        acc = acc.wrapping_add(i);
    }
    std::hint::black_box(acc);
    t.elapsed()
}

/// 对照：多个线程争用同一把 Mutex，临界区极短（只有一次 += 1），
/// 但每次进出临界区都要经过锁的原子操作，线程数越多竞争越激烈。
fn bench_mutex_contention() -> std::time::Duration {
    let counter = Arc::new(Mutex::new(0u64));
    let t = Instant::now();
    thread::scope(|s| {
        for _ in 0..THREADS {
            let counter = Arc::clone(&counter);
            s.spawn(move || {
                for _ in 0..ROUNDS_PER_THREAD {
                    let mut n = counter.lock().unwrap();
                    *n = n.wrapping_add(1);
                }
            });
        }
    });
    t.elapsed()
}

fn main() {
    logln!("--- 空循环 vs {THREADS} 线程争用同一把锁的短临界区 ---");
    logln!("每种方式总共完成 {} 次加法", ROUNDS_PER_THREAD * THREADS as u64);

    let dur_empty = bench_empty_loop();
    logln!("单线程空转加法：{dur_empty:?}");

    let dur_mutex = bench_mutex_contention();
    logln!("{THREADS} 线程争用 Mutex 做同等次数加法：{dur_mutex:?}");

    let ratio = dur_mutex.as_secs_f64() / dur_empty.as_secs_f64().max(1e-12);
    logln!("★ 带锁竞争版本慢了约 {ratio:.1}x（即使总加法次数相同）");
    logln!("  差的不是「加法」本身，而是：");
    logln!("  1) 每次 lock/unlock 都是原子操作，比普通内存读写贵；");
    logln!("  2) 多核争用同一把锁，锁内部状态所在的缓存行要在核心间来回同步；");
    logln!("  3) 竞争激烈时，拿不到锁的线程可能真被内核挂起，牵扯上下文切换（微秒级）——");
    logln!("     这正是「上下文切换的成本」一节说的「间接成本」在应用层的直接体现。");
}
