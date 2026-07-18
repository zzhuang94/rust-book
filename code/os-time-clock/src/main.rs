//! SystemTime vs Instant：测耗时该用哪个
//!
//! 配套文档：docs/concurrency/os-basics.md
//! 运行：cargo run -p os-time-clock（先 cd code）
//!
//! 标准库给了两种"时间"，用途完全不同：
//!   - `SystemTime`：挂钟时间（wall clock），对应"现在几点几分"，可以转成 Unix 时间戳，
//!     但它**不是单调的**——系统时间可能被 NTP 校准、被管理员手动调整，甚至往回跳；
//!   - `Instant`：单调时钟（monotonic clock），只保证"后取的值不小于先取的值"，
//!     不能转成日历时间，但**专门用来测耗时**——不受系统时间被改动的影响。
//!
//! 一句话：要「现在几点」用 SystemTime；要「这段代码跑了多久」用 Instant。

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use labkit::logln;

fn main() {
    logln!("--- SystemTime：适合回答「现在几点」 ---");
    let now = SystemTime::now();
    match now.duration_since(UNIX_EPOCH) {
        Ok(d) => logln!("SystemTime::now() 距 Unix 纪元 = {} 秒（可以换算成日历时间）", d.as_secs()),
        Err(e) => logln!("竟然早于 Unix 纪元？！：{e}（几乎不可能发生，除非系统时间设错了）"),
    }

    logln!("--- Instant：适合回答「这段代码跑了多久」 ---");
    let start = Instant::now();
    std::thread::sleep(Duration::from_millis(120));
    let elapsed = start.elapsed(); // Instant 内部只关心"流逝了多久"，不关心"现在几点"
    logln!("Instant 测得的耗时 ≈ {elapsed:?}（睡了 120ms，测出来应该非常接近）");

    logln!("--- 为什么不能用 SystemTime 测耗时：它可能被系统时钟调整影响 ---");
    // SystemTime::now() 两次调用相减，理论上可能因为系统时间被 NTP 向后调整而出现
    // "结束时间反而早于开始时间"的情况——这种情况下 duration_since 会返回 Err。
    let sys_start = SystemTime::now();
    std::thread::sleep(Duration::from_millis(50));
    let sys_end = SystemTime::now();
    match sys_end.duration_since(sys_start) {
        Ok(d) => logln!(
            "本次用 SystemTime 相减也算出了 {d:?}（正常情况下没问题，但这只是运气好）"
        ),
        Err(_) => logln!("★ 出现了！SystemTime 相减失败——系统时间在这段时间里被向回调整了"),
    }
    logln!("★ 结论：SystemTime 相减在理论上就可能失败（返回 Err），Instant 永远不会——");
    logln!("  因为 Instant 承诺单调递增，这是操作系统提供的一个专门保证，不依赖挂钟时间。");

    logln!("--- 小结 ---");
    logln!("  1) 记日志时间戳 / 存到数据库的时间 —— 用 SystemTime（或直接用 chrono）；");
    logln!("  2) 测耗时、超时判断、性能计时 —— 永远用 Instant；");
    logln!("  3) 这份仓库里的 labkit::logln! 内部用 chrono::Local::now()（本质是 SystemTime 的封装），");
    logln!("     因为它要打印「现在几点几分」，不是在测耗时——两者用途一致，选对了工具。");
}
