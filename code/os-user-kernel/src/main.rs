//! 用户态 vs 内核态：系统调用密集 vs 纯内存循环，成本差多少个数量级
//!
//! 配套文档：docs/concurrency/os-basics.md
//! 运行：cargo run -p os-user-kernel（先 cd code）
//!
//! CPU 有"用户态"（你的程序代码跑的地方，权限受限）和"内核态"（操作系统内核跑的地方，
//! 权限完全）两种模式。任何系统调用（读文件、创建线程、网络 IO……）都要从用户态"陷入"
//! 内核态，内核办完事再切回来——这一趟切换本身有固定开销，跟"业务逻辑复杂度"无关。
//!
//! Linux 上常见的教学写法是狂调 `getpid()` 或读 `/proc/self/stat`，但这份代码要跨平台
//! （包括 Windows），所以换一个所有平台都有、且明显是系统调用的操作：
//! "反复创建又删除一个临时文件"（每次都要经过内核的文件系统层）。
//! 拿它和一个等量级的纯内存循环（不触发任何系统调用）对比，看差几个数量级。

use std::fs;
use std::time::Instant;

use labkit::logln;

const ROUNDS: u64 = 2_000;

/// 系统调用密集：每一轮都创建一个文件再删除它——两次系统调用。
fn syscall_heavy(dir: &std::path::Path) -> std::time::Duration {
    let t = Instant::now();
    for i in 0..ROUNDS {
        let path = dir.join(format!("os_user_kernel_{i}.tmp"));
        fs::write(&path, b"x").expect("创建临时文件失败"); // 陷入内核：文件系统调用
        fs::remove_file(&path).expect("删除临时文件失败"); // 再陷入一次
    }
    t.elapsed()
}

/// 纯内存循环：数量级和上面对齐，但不触发任何系统调用，全程留在用户态。
fn pure_memory(rounds: u64) -> std::time::Duration {
    let t = Instant::now();
    let mut acc: u64 = 0;
    for i in 0..rounds {
        // 用点"看起来有实际工作"的运算，避免被编译器整个优化掉。
        acc = acc.wrapping_add(i.wrapping_mul(2654435761));
    }
    std::hint::black_box(acc);
    t.elapsed()
}

fn main() {
    let dir = std::env::temp_dir();
    logln!("--- 用户态 vs 内核态：系统调用密集 vs 纯内存循环 ---");
    logln!("对比 {ROUNDS} 次「创建+删除临时文件」 vs {ROUNDS} 次「纯内存运算」");

    let dur_syscall = syscall_heavy(&dir);
    logln!("系统调用密集（创建+删除文件 x{ROUNDS}）耗时: {dur_syscall:?}");

    let dur_memory = pure_memory(ROUNDS);
    logln!("纯内存循环（同等轮数）耗时: {dur_memory:?}");

    let ratio = dur_syscall.as_secs_f64() / dur_memory.as_secs_f64().max(1e-12);
    logln!("★ 系统调用密集版本慢了约 {ratio:.0}x");
    logln!("  这不是因为「创建文件」逻辑复杂，而是每次系统调用都要陷入内核态再返回，");
    logln!("  这一趟固定开销比纯用户态的内存运算贵好几个数量级——");
    logln!("  这也是为什么高性能程序会想方设法「批量化」系统调用（对照 os-disk-io 那一课）");
}
