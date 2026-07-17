//! 极简反向代理演示：前端 accept 客户端连接，代理自己再连一次后端 echo
//! 服务，双向转发数据，重点展示"经过代理之后，后端看到的对端地址
//! 其实是代理自己的地址，不是真实客户端"这件事。
//!
//! 跟真实反向代理（Nginx）比，这里只保留最核心的骨架：
//!   1. 前端 accept 到一条客户端连接，记下客户端的真实地址（peer）；
//!   2. 代理主动向后端发起一条新连接，这条新连接的本地地址就是
//!      "代理自己在本地用的地址"——后端 accept 到的正是这个地址；
//!   3. 把两条连接的数据双向搬运（tokio::io::copy 各管一个方向）。
//!
//! 配套文档：docs/network/proxy-nat.md
//! 运行（在 code/ 下）：cargo run -p network-proxy-nat

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::Context;
use labkit::logln;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

/// 最简单的 echo 后端：原样把收到的数据写回去，直到对端关闭连接。
///
/// 注意这里打印的 `peer`：由于连接是代理主动发起的，这个地址是
/// 代理自己的本地地址，不是最初那个真实客户端——这正是本课的核心现象。
async fn echo_backend(mut socket: TcpStream, peer: SocketAddr) -> anyhow::Result<()> {
    logln!("[后端] accept 到一条连接，peer = {peer}（这其实是代理的地址，不是真实客户端）");

    let mut buf = [0u8; 1024];
    loop {
        let n = socket.read(&mut buf).await.context("后端读取失败")?;
        if n == 0 {
            logln!("[后端] 连接 {peer} 已关闭");
            break;
        }
        socket.write_all(&buf[..n]).await.context("后端回写失败")?;
    }
    Ok(())
}

/// 后端主循环：一直 accept，每条连接单独 spawn 一个 echo 任务。
async fn run_backend(listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((socket, peer)) => {
                tokio::spawn(async move {
                    if let Err(err) = echo_backend(socket, peer).await {
                        logln!("[后端] 处理连接 {peer} 出错：{err:?}");
                    }
                });
            }
            Err(err) => {
                logln!("[后端] accept 失败，停止：{err:?}");
                break;
            }
        }
    }
}

/// 代理处理一条客户端连接：连后端 + 双向转发，转发前打印一组对照日志。
async fn handle_client(
    client: TcpStream,
    client_peer: SocketAddr,
    backend_addr: SocketAddr,
) -> anyhow::Result<()> {
    let backend = TcpStream::connect(backend_addr)
        .await
        .with_context(|| format!("代理连接后端 {backend_addr} 失败"))?;

    // 这个 local_addr 就是代理这条"连后端"的连接自己用的本地地址，
    // 后端 accept 到的 peer 会和这个地址完全一致——对照日志能直接看出
    // "客户端真实地址" 和 "后端看到的地址" 是两个不同的东西。
    let proxy_local_addr = backend.local_addr()?;
    logln!(
        "[代理] 前端 peer（真实客户端） = {client_peer}，连后端时代理本地地址 = {proxy_local_addr}"
    );

    let (mut client_r, mut client_w) = client.into_split();
    let (mut backend_r, mut backend_w) = backend.into_split();

    let client_to_backend = tokio::io::copy(&mut client_r, &mut backend_w);
    let backend_to_client = tokio::io::copy(&mut backend_r, &mut client_w);

    tokio::try_join!(client_to_backend, backend_to_client).context("双向转发中断")?;
    Ok(())
}

/// 代理前端主循环：accept 客户端连接，转给 `handle_client` 处理。
async fn run_proxy(listener: TcpListener, backend_addr: SocketAddr) {
    loop {
        match listener.accept().await {
            Ok((client, peer)) => {
                logln!("[代理] 前端收到客户端连接，peer = {peer}");
                tokio::spawn(async move {
                    if let Err(err) = handle_client(client, peer, backend_addr).await {
                        logln!("[代理] 转发客户端 {peer} 失败：{err:?}");
                    }
                });
            }
            Err(err) => {
                logln!("[代理] accept 失败，停止：{err:?}");
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logln!("=== 极简反向代理演示：转发 + 地址对照 ===");

    // 后端 echo 服务，端口 0 让操作系统分配。
    let backend_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("绑定后端失败")?;
    let backend_addr = backend_listener.local_addr()?;
    logln!("后端监听 {backend_addr}");
    tokio::spawn(run_backend(backend_listener));

    // 代理前端只暴露这一个地址，客户端完全不知道背后还有一台后端。
    let proxy_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("绑定代理前端失败")?;
    let proxy_addr = proxy_listener.local_addr()?;
    logln!("代理前端监听 {proxy_addr}");
    tokio::spawn(run_proxy(proxy_listener, backend_addr));

    // 给后台任务一点时间把 accept 循环跑起来，避免第一次连接偶发落空。
    tokio::time::sleep(Duration::from_millis(50)).await;

    // 客户端只知道代理地址，完全不知道真正处理请求的是另一台后端。
    let mut client = TcpStream::connect(proxy_addr)
        .await
        .context("客户端连接代理失败")?;
    let client_local_addr = client.local_addr()?;
    logln!("客户端连接代理，客户端自己的地址 = {client_local_addr}（这才是真实客户端地址）");

    client.write_all(b"hello via proxy").await?;

    let mut buf = [0u8; 128];
    let n = timeout(Duration::from_secs(2), client.read(&mut buf))
        .await
        .context("等待回显超时")?
        .context("读取回显失败")?;
    logln!("客户端收到回显：{:?}", String::from_utf8_lossy(&buf[..n]));

    client.shutdown().await.ok();
    tokio::time::sleep(Duration::from_millis(50)).await;

    logln!("演示结束：对照上面两行日志——");
    logln!("「客户端自己的地址」= {client_local_addr}，「后端看到的 peer」应该是代理本地地址，两者不同。");
    Ok(())
}
