//! 核数 + 伪共享（false sharing）粗略对比
//!
//! 配套文档：docs/concurrency/os-basics.md 「缓存与缓存行」
//! 运行：cargo run -p os-cpu-memory（先 cd code，建议 --release 跑，效果更明显）
//!
//! 缓存以"缓存行"（cache line，通常 64 字节）为单位在核心之间搬运。
//! 如果两个线程各自写**同一缓存行里的不同变量**，硬件的缓存一致性协议要不断把这行
//! 数据在两个核心之间搬来搬去——这叫"伪共享"（false sharing），逻辑上没有共享任何东西，
//! 代价却和真共享一样贵。
//!
//! 下面用两组 AtomicU64 对比：
//!   - "挤在一起"：两个 AtomicU64 紧挨着放（一个 u64 = 8 字节，两个只占 16 字节，
//!     必然落在同一条 64 字节缓存行里）；
//!   - "隔开"：中间垫 64 字节的 padding，强制它们落在不同缓存行。
//! 两组各开两个线程狂加，比较耗时。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use labkit::logln;

const ROUNDS: u64 = 20_000_000;

/// 两个计数器紧挨着放：几乎必然落在同一条缓存行里 → 伪共享。
#[repr(C)]
struct Cramped {
    a: AtomicU64,
    b: AtomicU64,
}

/// 手动用 padding 把两个计数器分隔到不同缓存行（64 字节对齐）。
/// #[repr(align(64))] 强制整个结构体按 64 字节对齐，
/// 结构体里第二个字段自然被挤到下一条缓存行去。
#[repr(C, align(64))]
struct PaddedA {
    v: AtomicU64,
}
#[repr(C, align(64))]
struct PaddedB {
    v: AtomicU64,
}

fn bench_cramped() -> std::time::Duration {
    let shared = Arc::new(Cramped { a: AtomicU64::new(0), b: AtomicU64::new(0) });
    let t = Instant::now();
    thread::scope(|s| {
        let s1 = Arc::clone(&shared);
        s.spawn(move || for _ in 0..ROUNDS { s1.a.fetch_add(1, Ordering::Relaxed); });
        let s2 = Arc::clone(&shared);
        s.spawn(move || for _ in 0..ROUNDS { s2.b.fetch_add(1, Ordering::Relaxed); });
    });
    t.elapsed()
}

fn bench_padded() -> std::time::Duration {
    let a = Arc::new(PaddedA { v: AtomicU64::new(0) });
    let b = Arc::new(PaddedB { v: AtomicU64::new(0) });
    let t = Instant::now();
    thread::scope(|s| {
        let a = Arc::clone(&a);
        s.spawn(move || for _ in 0..ROUNDS { a.v.fetch_add(1, Ordering::Relaxed); });
        let b = Arc::clone(&b);
        s.spawn(move || for _ in 0..ROUNDS { b.v.fetch_add(1, Ordering::Relaxed); });
    });
    t.elapsed()
}

fn main() {
    let n = thread::available_parallelism().map(|p| p.get()).unwrap_or(1);
    logln!("--- 核数 ---");
    logln!("available_parallelism() = {n}（逻辑核数，超线程也算在内）");
    if n < 2 {
        logln!("★ 这台机器只有 1 个可用逻辑核，下面的双线程对比意义不大，但仍会跑");
    }

    logln!("--- 伪共享 vs 隔开：两个线程各自狂加各自的计数器 ---");
    logln!("（提示：debug 模式下原子操作本身开销就大，差异可能被掩盖；建议 --release 观察）");

    // 各跑两遍简单取更稳定的一次感觉（教学 demo，非严格 benchmark）。
    let cramped = bench_cramped();
    let padded = bench_padded();

    logln!("挤在一起（同一缓存行，伪共享）：{cramped:?}");
    logln!("隔开 64 字节（各占一条缓存行）：{padded:?}");

    if cramped > padded {
        let ratio = cramped.as_secs_f64() / padded.as_secs_f64();
        logln!("★ 挤在一起慢了约 {ratio:.2}x —— 这就是缓存行在核心间来回“弹跳”的代价");
    } else {
        logln!("★ 本次没测出明显差异（现代 CPU/编译器优化、调度抖动都会影响，属于正常噪声）");
        logln!("  但结论本身成立：ArcSwap 一章提到的“读锁计数器在多核间弹跳”就是这个物理原型");
    }
}
