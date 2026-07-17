//! 极简 HTTP/1.1 协议演示：不借助任何 HTTP 库，手写"读请求头"和"拼响应"，
//! 让读者看清 HTTP 报文本质上就是一段格式固定的纯文本，跑在 TCP 字节流之上。
//! 跟 network-tcp 课的关系：TCP 没有消息边界，HTTP 报文自己用 `\r\n\r\n`
//! 划出"请求头结束"的边界，这就是应用层协议解决"粘包"问题的一种方式。
//!
//! 运行（在 code/ 下）：cargo run -p network-http-protocol

use anyhow::Context;
use labkit::logln;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// 手写的固定 HTTP/1.1 响应：状态行 + 两个响应头 + 空行 + 响应体。
/// 真实服务器会按请求内容动态生成，这里故意写死，突出"协议长什么样"这件事。
const RESPONSE: &str =
    "HTTP/1.1 200 OK\r\nContent-Length: 13\r\nContent-Type: text/plain\r\n\r\nHello, HTTP!\n";

/// 在字节缓冲里找 `\r\n\r\n` 的位置——这是 HTTP 报文里"请求头结束"的边界标记。
fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// 服务端：只做"读到请求头结束标记就回一个固定响应"，不解析请求行、
/// 不区分 method/path，聚焦在"协议边界在哪里"这一件事上。
async fn serve_one(mut socket: TcpStream) -> anyhow::Result<()> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 512];

    loop {
        let n = socket.read(&mut chunk).await.context("读取请求失败")?;
        if n == 0 {
            anyhow::bail!("客户端提前关闭了连接，还没读到完整请求头");
        }
        buf.extend_from_slice(&chunk[..n]);

        if let Some(pos) = find_header_end(&buf) {
            let headers = String::from_utf8_lossy(&buf[..pos]);
            logln!("服务端读到完整请求头：\n{headers}");
            break;
        }
    }

    socket
        .write_all(RESPONSE.as_bytes())
        .await
        .context("回写响应失败")?;
    // 主动关闭写端：客户端请求里带了 `Connection: close`，这里配合它，
    // 让客户端的 read_to_end 能在读完响应后拿到 EOF 而不是一直等下去。
    socket.shutdown().await.ok();
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logln!("=== 手写最简 HTTP/1.1 协议演示 ===");

    // 127.0.0.1:0：只监听本机环回地址，端口交给操作系统分配。
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("绑定监听地址失败")?;
    let addr = listener.local_addr()?;
    logln!("HTTP 服务监听 {addr}");

    let server = tokio::spawn(async move {
        let (socket, peer) = listener.accept().await?;
        logln!("服务端 accept 到连接，peer = {peer}");
        serve_one(socket).await
    });

    // 客户端：手写一个最简 GET 请求，格式跟浏览器/curl 发出的报文完全一致。
    let mut client = TcpStream::connect(addr).await.context("客户端连接失败")?;
    let request = format!("GET / HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    client
        .write_all(request.as_bytes())
        .await
        .context("发送请求失败")?;
    logln!("客户端发出请求：\n{request}");

    // 把服务端回写的原始字节整段读出来再打印，展示"协议原文"到底长什么样。
    let mut raw = Vec::new();
    client
        .read_to_end(&mut raw)
        .await
        .context("读取响应失败")?;
    logln!(
        "客户端收到原始响应字节（共 {} 字节）：\n{}",
        raw.len(),
        String::from_utf8_lossy(&raw)
    );

    server.await??;
    Ok(())
}
