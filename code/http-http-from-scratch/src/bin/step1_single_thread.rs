//! 第 0 步：不用 tokio、不用 axum，纯标准库手写一个最小 HTTP 服务器。
//! （单线程阻塞版 —— 它有一个致命缺陷，跑起来亲手体会）
//!
//! 运行：cargo run -p http-http-from-scratch --bin step1_single_thread
//! 试验：
//!   终端A: curl http://127.0.0.1:7080/          → 立即返回
//!   终端A: curl http://127.0.0.1:7080/slow      → 5 秒后返回
//!   ★ 关键实验：先 curl /slow，趁它没返回，另开终端B curl / ——
//!     终端B 也要等 5 秒！因为服务器是单线程的，一次只能伺候一个连接。
//!
//! 对照 Go：你从没写过这一层，因为 net/http 把它全包了。
//! 这一步的意义就是掀开地毯，看看 HTTP 服务器的地基长什么样。

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use labkit::logln;

fn main() {
    // (1) 监听 TCP 端口。HTTP 服务器的本质：一个 TCP 服务器，说的话符合 HTTP 格式。
    //     对照 Go：ln, _ := net.Listen("tcp", ":7080")
    let listener = TcpListener::bind("0.0.0.0:7080").unwrap();
    logln!("第 0 步·单线程阻塞版已启动：http://127.0.0.1:7080");
    logln!("试试 curl /slow 的同时另开终端 curl / —— 感受'单线程'的痛\n");

    // (2) incoming()：一个"接连接"的迭代器，每来一个 TCP 连接产出一个 stream。
    //     对照 Go：for { conn, _ := ln.Accept(); ... }
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        // (3) 致命缺陷在这里：handle 是普通函数调用，处理完这个连接
        //     才会回到循环去 accept 下一个 —— 同一时刻只能伺候一个人！
        handle(stream);
    }
}

/// 处理一个连接：读请求 → 解析 → 造响应 → 写回去。全是同步阻塞操作。
fn handle(mut stream: TcpStream) {
    // (4) 从 TCP 连接里读原始字节。HTTP/1.1 请求就是一段有格式的文本：
    //
    //     GET /slow HTTP/1.1\r\n          ← 请求行：方法 路径 版本
    //     Host: 127.0.0.1:7080\r\n        ← 请求头，每行一个
    //     User-Agent: curl/8.0\r\n
    //     \r\n                            ← 空行 = 头结束（之后是可选的 body）
    //
    //     （\r\n 是"回车+换行"，HTTP 规定的行分隔符）
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).unwrap_or(0); // 阻塞：没数据就干等
    let request = String::from_utf8_lossy(&buf[..n]);

    // 打印请求的前几行，亲眼看看 curl 到底发了什么
    let preview: Vec<&str> = request.lines().take(3).collect();
    logln!("收到请求（前 3 行）: {preview:?}");

    // (5) 手工"路由"：从请求行 "GET /slow HTTP/1.1" 里抠出路径（第 2 个空格分隔段）
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/")
        .to_string();

    // (6) 手工"handler"：按路径决定状态码和响应体
    let (status, body) = match path.as_str() {
        "/" => ("200 OK", "hello，这是一份手写的 HTTP 响应".to_string()),
        "/slow" => {
            logln!("处理 /slow：同步睡 5 秒（整个服务器都卡住了！）");
            std::thread::sleep(std::time::Duration::from_secs(5));
            ("200 OK", "慢请求完成".to_string())
        }
        _ => ("404 Not Found", format!("没有这个路径: {path}")),
    };

    // (7) 手工拼 HTTP 响应。响应格式和请求对称：
    //
    //     HTTP/1.1 200 OK\r\n             ← 状态行
    //     Content-Type: ...\r\n           ← 响应头
    //     Content-Length: 33\r\n          ← body 的【字节】数，浏览器靠它知道读多少
    //     \r\n                            ← 空行 = 头结束
    //     hello，这是一份手写的 HTTP 响应   ← body
    //
    //     注意 Content-Length 是字节数：String::len() 给的正是字节数
    //     （一个汉字 UTF-8 占 3 字节），所以直接用 body.len()。
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).unwrap();
    // stream 在函数结尾 drop → TCP 连接关闭（我们声明了 Connection: close）
}
