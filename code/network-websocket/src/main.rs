//! WebSocket echo demo：用 tokio-tungstenite 同进程起「服务端 + 客户端」。
//!
//! WebSocket 本质是：先走一次 HTTP Upgrade 握手，把一条 TCP 连接升级成
//! 全双工的消息帧通道，之后双方就可以互相随时推消息（不再是 请求⇄响应 那套）。
//!
//! 只监听 127.0.0.1，端口传 0 让操作系统挑一个空闲端口再回读，避免固定端口冲突。
//!
//! 运行（在 code/ 下）：cargo run -p network-websocket

use std::net::SocketAddr;

use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use labkit::logln;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

/// 服务端逻辑：接受一条连接、完成 WebSocket 握手，然后原样回显收到的文本消息。
async fn run_server(listener: TcpListener) -> anyhow::Result<()> {
    let (stream, peer) = listener.accept().await?;
    logln!("服务端收到 TCP 连接：{peer}");

    // accept_async 在这条已建立的 TCP 流上完成 WebSocket 握手（HTTP Upgrade）。
    let mut ws = tokio_tungstenite::accept_async(stream)
        .await
        .context("WebSocket 握手失败")?;
    logln!("服务端握手完成，进入 echo 循环");

    while let Some(msg) = ws.next().await {
        match msg? {
            Message::Text(text) => {
                logln!("服务端收到文本：{text}");
                ws.send(Message::Text(text)).await?;
            }
            Message::Close(_) => {
                logln!("服务端收到关闭帧，退出 echo 循环");
                break;
            }
            // Ping/Pong/Binary 等帧本课不关心，直接忽略。
            _ => {}
        }
    }
    Ok(())
}

/// 客户端逻辑：连接、发一条文本消息、等回显、再主动关闭。
async fn run_client(addr: SocketAddr) -> anyhow::Result<()> {
    let url = format!("ws://{addr}/");
    let (mut ws, _handshake_response) = tokio_tungstenite::connect_async(&url)
        .await
        .context("客户端连接失败")?;
    logln!("客户端已连接：{url}");

    ws.send(Message::text("hello websocket")).await?;
    logln!("客户端已发送：hello websocket");

    match ws.next().await {
        Some(Ok(Message::Text(text))) => logln!("客户端收到回显：{text}"),
        Some(Ok(other)) => logln!("客户端收到非文本帧：{other:?}"),
        Some(Err(err)) => return Err(err.into()),
        None => logln!("客户端没收到任何回复（连接被对端关闭）"),
    }

    // 主动发关闭帧，走完 WebSocket 的正常收尾流程。
    ws.close(None).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    logln!("WebSocket 服务监听 {addr}");

    // 服务端放进后台任务跑；主任务扮演客户端，两端同进程演示握手全过程。
    let server = tokio::spawn(run_server(listener));

    run_client(addr).await?;

    server.await??;
    Ok(())
}
