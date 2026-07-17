//! 第 2 步：把"每连接一个线程"换成"每连接一个 tokio 任务"。
//! 结构和 step2 一模一样，只是三处换成了异步版本 —— 逐行对比着看：
//!   std::net::TcpListener        → tokio::net::TcpListener（accept 是 async）
//!   thread::spawn(闭包)          → tokio::spawn(async 块)     （01/02 课）
//!   Read/Write + thread::sleep   → AsyncReadExt/AsyncWriteExt + tokio 的 sleep
//!
//! 运行：cargo run -p http-http-from-scratch --bin step3_tokio_task
//! 试验：和 step2 相同（/slow 不挡 /），但每个连接的开销从
//! 一个 OS 线程（栈 ~8MB、内核调度）降到一个异步任务（几百字节的状态机）。
//! 一万并发连接 = 一万个小状态机躺在少数几个工作线程上 —— 这就是 C10K 的异步答案，
//! 也是你用第 01~03 课的全部知识，亲手造出的异步 HTTP 服务器。

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use labkit::logln;

#[tokio::main]
async fn main() {
    // tokio 版 TcpListener：bind/accept 都是异步的（等连接时让出线程）
    let listener = TcpListener::bind("0.0.0.0:7080").await.unwrap();
    logln!("第 2 步·tokio 任务版已启动：http://127.0.0.1:7080");
    logln!("每个连接一个异步任务（几百字节），而不是一个 OS 线程（~8MB）\n");

    loop {
        // accept()：等下一个 TCP 连接。返回 (连接, 对端地址)。
        // 对照 step2 的 listener.incoming() 迭代器，这里是显式循环 + await。
        let (stream, addr) = listener.accept().await.unwrap();
        // ★ 每连接一个任务：spawn 立即返回，主循环马上回去 accept 下一个。
        //   对照 step2 的 thread::spawn(move || handle(stream))。
        tokio::spawn(async move {
            logln!("接到连接: {addr}");
            handle(stream).await;
        });
    }
}

/// 处理逻辑与 step1/step2 完全相同，只是读写和睡眠换成异步版。
async fn handle(mut stream: TcpStream) {
    let mut buf = [0u8; 4096];
    // 异步读：数据没到就让出线程去伺候别的连接（step2 里是干等）
    let n = stream.read(&mut buf).await.unwrap_or(0);
    let request = String::from_utf8_lossy(&buf[..n]);

    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/")
        .to_string();

    let (status, body) = match path.as_str() {
        "/" => ("200 OK", "hello，来自 tokio 任务版服务器".to_string()),
        "/slow" => {
            logln!("/slow 异步睡 5 秒（只挂起自己这个任务，线程照常伺候别人）");
            // 注意：这里必须用 tokio 的异步 sleep！
            // 用 std::thread::sleep 会卡住工作线程（第 01 课的铁律）。
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            ("200 OK", "慢请求完成".to_string())
        }
        _ => ("404 Not Found", format!("没有这个路径: {path}")),
    };

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
}
