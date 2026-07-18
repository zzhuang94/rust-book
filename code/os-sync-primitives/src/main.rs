//! Arc<Mutex<T>>：两个线程安全地累加同一个计数器
//!
//! 配套文档：docs/concurrency/os-basics.md 「线程共享地址空间」
//! 运行：cargo run -p os-sync-primitives（先 cd code）
//!
//! 同进程的线程共享同一份内存——这是福（共享零成本）也是祸（数据竞争）。
//! `Arc<Mutex<T>>` 是最朴素的解法：
//!   - `Arc`（Atomically Reference Counted）负责"多个线程都能持有这份数据的所有权"；
//!   - `Mutex`（互斥锁）负责"同一时刻只允许一个线程真正读写里面的数据"。
//! 下面开两个线程，各自对同一个计数器累加 100000 次，最后校验结果精确等于 200000——
//! 如果没有 Mutex，两个线程同时读改写同一个 int，会发生"数据竞争"，结果会小于 200000
//! （某些加法会被"吞掉"）。

use std::sync::{Arc, Mutex};
use std::thread;

use labkit::logln;

const PER_THREAD: u64 = 100_000;

fn main() {
    logln!("--- Arc<Mutex<u64>>：两个线程累加同一个计数器 ---");

    // Arc::new 把计数器包成"可以被多个线程共享所有权"的形式；
    // Mutex::new 再包一层"读写它之前必须先拿锁"。
    let counter = Arc::new(Mutex::new(0u64));

    let counter_a = Arc::clone(&counter); // clone 只是给引用计数 +1，不是深拷贝数据
    let handle_a = thread::spawn(move || {
        for _ in 0..PER_THREAD {
            // lock() 拿到一个 MutexGuard；这行结束（语句末尾）guard 就 drop = 自动解锁。
            // 这一步是"临界区"：拿锁 -> 改数据 -> 释放锁，中间不会被另一个线程插入。
            let mut n = counter_a.lock().unwrap();
            *n += 1;
        }
        logln!("  线程 A 累加完成");
    });

    let counter_b = Arc::clone(&counter);
    let handle_b = thread::spawn(move || {
        for _ in 0..PER_THREAD {
            let mut n = counter_b.lock().unwrap();
            *n += 1;
        }
        logln!("  线程 B 累加完成");
    });

    handle_a.join().unwrap();
    handle_b.join().unwrap();

    let final_value = *counter.lock().unwrap();
    let expected = PER_THREAD * 2;
    logln!("最终计数 = {final_value}（期望 = {expected}）");

    if final_value == expected {
        logln!("★ 精确相等：Mutex 保证了每次 += 1 都是「读-改-写」一整个不可分割的动作");
    } else {
        // 理论上不会走到这个分支（Mutex 保证正确性），留着只是让读者看到"如果错了会怎样"。
        logln!("★ 出乎意料：出现了少加的情况，说明同步没有生效（不应该发生）");
    }

    logln!("对照：如果去掉 Mutex，直接两个线程同时 += 一个裸的 u64，就是典型的「数据竞争」——");
    logln!("Rust 的 Send/Sync trait 在编译期就会拒绝你这么写，这是它和很多语言的关键区别。");
}
