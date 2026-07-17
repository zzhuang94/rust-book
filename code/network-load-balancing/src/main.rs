//! 极简四层负载均衡演示：本进程起 2 个后端 echo 服务，前端按轮询把
//! 客户端连接转发到其中一个后端，双向转发数据（后端负责回显）。
//!
//! 跟真实 LB（Nginx/HAProxy/云 LB）比，这里只保留最核心的骨架：
//!   1. 维护一份后端地址列表 + 一个轮询游标；
//!   2. 每来一个新连接，按游标选一台后端，游标往前挪一格；
//!   3. 自己连一条新连接到选中的后端，把两条连接的数据双向搬运。
//!
//! 配套文档：docs/network/load-balancing.md
//! 运行（在 code/ 下）：cargo run -p network-load-balancing

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::Context;
use labkit::logln;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

/// 一个最简单的 echo 后端：原样把收到的数据写回去，直到对端关闭连接。
async fn echo_backend(id: usize, mut socket: TcpStream, peer: SocketAddr) -> anyhow::Result<()> {
    // 注意：这里的 peer 是"谁连过来的"，在有前端转发的情况下，
    // 这其实是前端自己的地址，不是真实客户端——下一课《代理与 NAT》细讲。
    logln!("后端 {id} 收到连接，来自 {peer}");

    let mut buf = [0u8; 1024];
    loop {
        let n = socket.read(&mut buf).await.context("后端读取失败")?;
        if n == 0 {
            logln!("后端 {id} 的连接 {peer} 已关闭");
            break;
        }
        socket.write_all(&buf[..n]).await.context("后端回写失败")?;
    }
    Ok(())
}

/// 后端主循环：一直 accept，每条连接单独 spawn 一个 echo 任务，互不阻塞。
async fn run_backend(id: usize, listener: TcpListener) {
    loop {
        match listener.accept().await {
            Ok((socket, peer)) => {
                tokio::spawn(async move {
                    if let Err(err) = echo_backend(id, socket, peer).await {
                        logln!("后端 {id} 处理连接 {peer} 出错：{err:?}");
                    }
                });
            }
            Err(err) => {
                logln!("后端 {id} accept 失败，停止该后端：{err:?}");
                break;
            }
        }
    }
}

/// 把一条客户端连接和一条后端连接绑在一起，双向转发数据。
///
/// `tokio::io::copy` 只负责一个方向；两个方向各起一份，用 `try_join!`
/// 让它们并发跑，任意一个方向读到 EOF、两边关闭时整体结束。
async fn forward(mut client: TcpStream, backend_addr: SocketAddr) -> anyhow::Result<()> {
    let mut backend = TcpStream::connect(backend_addr)
        .await
        .with_context(|| format!("连接后端 {backend_addr} 失败"))?;

    let (mut client_r, mut client_w) = client.split();
    let (mut backend_r, mut backend_w) = backend.split();

    let client_to_backend = tokio::io::copy(&mut client_r, &mut backend_w);
    let backend_to_client = tokio::io::copy(&mut backend_r, &mut client_w);

    tokio::try_join!(client_to_backend, backend_to_client).context("双向转发中断")?;
    Ok(())
}

/// 前端主循环：轮询挑后端 + 转发。轮询游标 `next` 只在这一个任务里
/// 顺序访问，不需要加锁——这也是"单个任务内部状态"和"多任务共享
/// 状态需要 Arc<Mutex<..>>"的区别，具体可参考《共享状态》一课。
async fn run_frontend(listener: TcpListener, backends: Vec<SocketAddr>) {
    let mut next = 0usize;
    loop {
        match listener.accept().await {
            Ok((client, peer)) => {
                let idx = next;
                let backend_addr = backends[idx];
                next = (next + 1) % backends.len();

                logln!("前端收到客户端 {peer}，轮询选中后端 {idx}（{backend_addr}）");

                tokio::spawn(async move {
                    if let Err(err) = forward(client, backend_addr).await {
                        logln!("转发客户端 {peer} 到后端 {idx} 失败：{err:?}");
                    }
                });
            }
            Err(err) => {
                logln!("前端 accept 失败，停止服务：{err:?}");
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logln!("=== 极简负载均衡：轮询 + 转发 + 回显 ===");

    // 启动两个后端，端口 0 让操作系统各自分配一个空闲端口。
    const BACKEND_COUNT: usize = 2;
    let mut backend_addrs = Vec::with_capacity(BACKEND_COUNT);
    for id in 0..BACKEND_COUNT {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .with_context(|| format!("绑定后端 {id} 失败"))?;
        let addr = listener.local_addr()?;
        backend_addrs.push(addr);
        logln!("后端 {id} 监听 {addr}");
        tokio::spawn(run_backend(id, listener));
    }

    // 前端只暴露这一个地址，客户端完全不需要知道后端列表。
    let frontend_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("绑定前端失败")?;
    let frontend_addr = frontend_listener.local_addr()?;
    logln!("前端监听 {frontend_addr}，后端列表 = {backend_addrs:?}");
    tokio::spawn(run_frontend(frontend_listener, backend_addrs));

    // 给后台任务一点时间把 accept 循环跑起来，避免第一次连接偶发落空。
    tokio::time::sleep(Duration::from_millis(50)).await;

    // 客户端连续发起 4 次请求，观察日志里轮询是否严格交替选后端。
    for i in 0..4u32 {
        let mut client = TcpStream::connect(frontend_addr)
            .await
            .with_context(|| format!("第 {i} 次连接前端失败"))?;

        let msg = format!("hello-{i}");
        client.write_all(msg.as_bytes()).await?;

        let mut buf = [0u8; 128];
        let n = timeout(Duration::from_secs(2), client.read(&mut buf))
            .await
            .context("等待回显超时")?
            .context("读取回显失败")?;
        logln!(
            "客户端第 {i} 次请求发出 {msg:?}，收到回显 {:?}",
            String::from_utf8_lossy(&buf[..n])
        );

        // 主动关闭写端，让后端读到 EOF，干净结束这条连接对应的任务。
        client.shutdown().await.ok();
    }

    logln!("演示结束：4 次请求应该严格交替落在后端 0 / 1 上");
    Ok(())
}
