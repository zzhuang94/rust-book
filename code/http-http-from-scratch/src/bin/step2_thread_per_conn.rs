//! 第 1 步：每个连接开一个线程 —— 这就是 Go net/http 的模型。
//! （相比第 0 步只改了一行：handle 用 thread::spawn 包起来）
//!
//! 运行：cargo run -p http-http-from-scratch --bin step2_thread_per_conn
//! 试验：
//!   先 curl /slow，趁它没返回，另开终端 curl / ——
//!   ★ 这次 / 立即返回了！慢请求在自己的线程里睡，不挡别人。
//!
//! 但线程不是免费的：
//!   - 每个 OS 线程默认栈 ~8MB（虚拟内存）、创建/切换都要过内核；
//!   - 一万个并发连接 = 一万个线程 → 调度和内存开销爆炸（经典的 C10K 问题）。
//! Go 的解法：goroutine（用户态轻量线程，几 KB 起步）—— 所以 Go 敢每连接一个 goroutine；
//! Rust 的解法：异步任务（第 01~03 课学的全部内容）—— 见下一步 step3。

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use labkit::logln;

fn main() {
    let listener = TcpListener::bind("0.0.0.0:7080").unwrap();
    logln!("第 1 步·线程版已启动：http://127.0.0.1:7080");
    logln!("这次 curl /slow 不会挡住别人了（每连接一个线程）\n");

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        // ★ 与第 0 步唯一的区别：开线程处理，主循环立刻回去 accept 下一个。
        //   对照 Go：go handleConn(conn) —— net/http 内部就是这么干的
        //   （只不过 Go 开的是廉价的 goroutine，我们开的是昂贵的 OS 线程）。
        thread::spawn(move || handle(stream));
    }
}

/// 和第 0 步一模一样的处理逻辑（读 → 解析 → 造响应 → 写）。
fn handle(mut stream: TcpStream) {
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).unwrap_or(0);
    let request = String::from_utf8_lossy(&buf[..n]);

    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/")
        .to_string();
    logln!("[线程 {:?}] 处理 {path}", thread::current().id());

    let (status, body) = match path.as_str() {
        "/" => ("200 OK", "hello，来自线程版服务器".to_string()),
        "/slow" => {
            logln!("[线程 {:?}] /slow 睡 5 秒（只卡自己这个线程）", thread::current().id());
            thread::sleep(std::time::Duration::from_secs(5));
            ("200 OK", "慢请求完成".to_string())
        }
        _ => ("404 Not Found", format!("没有这个路径: {path}")),
    };

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}
