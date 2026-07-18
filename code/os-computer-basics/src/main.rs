//! 计算机 = 一台不停执行指令的机器 —— 最基础的三个直觉
//!
//! 配套文档：docs/concurrency/os-basics.md 「CPU 一次一件事」「多核与超线程」
//! 运行：cargo run -p os-computer-basics（先 cd code）
//!
//! 这里不连网络、不开线程，只做三件"看得见"的事：
//!   1. 问操作系统"这台机器有多少个可并行执行的核"；
//!   2. 看一眼这台机器的指针宽度（64 位机器上一个指针占 8 字节）；
//!   3. 用一个傻乎乎的计数循环证明"程序真的在一条指令一条指令地跑"——
//!      不是玄学，是寄存器里的数字在被老老实实地累加。

use labkit::logln;

fn main() {
    logln!("--- 这台机器长什么样 ---");

    // std::thread::available_parallelism() 问操作系统："建议我开几个并行的执行流？"
    // 返回的是**逻辑核数**（物理核 + 超线程），不是物理核数。
    // 对照 Go：runtime.NumCPU()；这也是 Tokio 多线程运行时默认开的工作线程数。
    match std::thread::available_parallelism() {
        Ok(n) => logln!("available_parallelism() = {n} —— 操作系统认为这台机器能并行跑 {n} 条指令流"),
        Err(e) => logln!("available_parallelism() 查询失败：{e}（不常见，通常是权限问题）"),
    }

    // 指针大小 = 这台机器的"地址位宽"。64 位机器上一个指针（内存地址）占 8 字节，
    // 意味着一个进程最多能编号 2^64 个字节的虚拟地址——这是"虚拟内存"章节的地基。
    logln!(
        "size_of::<usize>() = {} 字节 —— 这是一个 {} 位机器",
        std::mem::size_of::<usize>(),
        std::mem::size_of::<usize>() * 8
    );

    // ---- 用一个傻乎乎的循环证明"程序在跑" ----
    // CPU 的本质是"取指令 → 执行 → 取下一条"的循环机器（docs 里的第一句话）。
    // 下面这个循环没有任何系统调用、没有任何 IO，纯粹是 CPU 在寄存器里反复加法。
    // 计时它能告诉你：CPU 每秒能做多少次这种最廉价的操作（数量级直觉，不是精确 benchmark）。
    logln!("--- 证明程序在一条一条地执行指令（纯计数循环） ---");
    let rounds: u64 = 200_000_000;
    let t = std::time::Instant::now();
    let mut acc: u64 = 0;
    for i in 0..rounds {
        // 用 wrapping_add 避免溢出 panic；这一行就是"CPU 在执行的指令"本体。
        acc = acc.wrapping_add(i);
    }
    let elapsed = t.elapsed();
    logln!(
        "累加 {rounds} 次，结果 = {acc}，耗时 {elapsed:?} —— 平均每次加法 {:.2} 纳秒",
        elapsed.as_nanos() as f64 / rounds as f64
    );
    logln!("★ 这几纳秒/次，就是「CPU 一次一件事」在你机器上的具体数字");
}
