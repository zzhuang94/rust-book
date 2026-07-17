//! 并发基础 · Rust 多线程与并发 —— 可运行示例
//!
//! 配套文档：docs/concurrency/threads.md
//! 运行：cargo run -p concurrency-threads（先 cd code）
//! 全部标准库，不需要 tokio。

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use labkit::logln;

fn main() {
    demo_spawn_join();
    demo_move();
    demo_arc_mutex();
    demo_mpsc();
    demo_scope();
    demo_parallel_sum();
}

fn demo_spawn_join() {
    logln!("--- spawn 与 join ---");
    let handle = thread::spawn(|| {
        for i in 1..=3 {
            logln!("  子线程: 第 {i} 步");
            thread::sleep(Duration::from_millis(30));
        }
        42 // 闭包返回值 = 线程结果
    });
    logln!("  主线程：子线程在后台跑");
    let result = handle.join().unwrap(); // 等它结束并取回返回值
    logln!("  子线程返回了 {result}");
}

fn demo_move() {
    logln!("--- move 把数据带进线程 ---");
    let data = vec![1, 2, 3];
    let h = thread::spawn(move || logln!("  子线程拿到: {data:?}")); // move 转移所有权
    h.join().unwrap();
}

fn demo_arc_mutex() {
    logln!("--- Arc + Mutex 共享计数 ---");
    let counter = Arc::new(Mutex::new(0u64));
    let mut handles = Vec::new();
    for _ in 0..4 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                *counter.lock().unwrap() += 1; // 语句结束 guard drop = 解锁
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    logln!("  结果 = {}（精确 4000）", *counter.lock().unwrap());
}

fn demo_mpsc() {
    logln!("--- mpsc 消息传递 ---");
    let (tx, rx) = mpsc::channel::<String>();
    for id in 1..=2 {
        let tx = tx.clone();
        thread::spawn(move || {
            for n in 1..=3 {
                tx.send(format!("生产者{id} 的第 {n} 条")).unwrap();
                thread::sleep(Duration::from_millis(10));
            }
        });
    }
    drop(tx); // 丢掉原始 tx，否则通道不关
    for msg in rx {
        logln!("  收到: {msg}");
    }
    logln!("  通道关闭");
}

fn demo_scope() {
    logln!("--- 作用域线程（借用栈数据）---");
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let mid = data.len() / 2;
    let (left, right) = data.split_at(mid);
    let (sum_l, sum_r) = thread::scope(|s| {
        let h1 = s.spawn(|| left.iter().sum::<i64>()); // 借用 left，不需要 move/Arc
        let h2 = s.spawn(|| right.iter().sum::<i64>());
        (h1.join().unwrap(), h2.join().unwrap())
    });
    logln!("  data 还能用: {data:?}，两半和 = {sum_l} + {sum_r} = {}", sum_l + sum_r);
}

fn demo_parallel_sum() {
    logln!("--- CPU 密集切块并行 ---");
    let data: Vec<i64> = (1..=1_000_000).collect();
    let n = thread::available_parallelism().map(|p| p.get()).unwrap_or(4);
    let chunk_size = data.len().div_ceil(n);
    let t = std::time::Instant::now();
    let total: i64 = thread::scope(|s| {
        let handles: Vec<_> = data.chunks(chunk_size).map(|chunk| s.spawn(move || chunk.iter().sum::<i64>())).collect();
        handles.into_iter().map(|h| h.join().unwrap()).sum()
    });
    logln!("  {n} 线程并行求和 = {total}，耗时 {:?}", t.elapsed());
}
