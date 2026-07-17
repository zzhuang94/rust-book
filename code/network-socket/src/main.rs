//! socket 五元组直觉课：`TcpListener::bind` + `accept`，配上 `TcpStream::connect`。
//!
//! 五元组 = (协议, 本地IP, 本地端口, 远端IP, 远端端口)，是操作系统用来唯一标识
//! 一条 TCP 连接的东西。这一课不传输任何业务数据，只把"连接建立后，
//! 两端各自看到什么地址"打印出来，建立起"一条连接 = 两组地址"的直觉。
//!
//! 运行（在 code/ 下）：cargo run -p network-socket

use anyhow::Context;
use labkit::logln;
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 127.0.0.1:0 —— 只监听本机环回地址，端口 0 让操作系统挑一个当前空闲端口，
    // 这样示例可以在任何机器上反复运行，不会因为"端口被占用"而失败。
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("绑定监听地址失败")?;
    let server_addr = listener.local_addr()?;
    logln!("服务端已监听：{server_addr}");

    // 服务端在后台任务里等待一个连接进来（accept 会一直阻塞/挂起，直到有连接）。
    let accept_task = tokio::spawn(async move {
        let (socket, client_addr_from_accept) = listener.accept().await?;
        // accept() 返回值里带的对端地址，理论上应该和 socket.peer_addr() 一致，
        // 这里两者都打出来做个对照，帮助建立"地址从哪来"的直觉。
        logln!("服务端 accept() 返回的对端地址 = {client_addr_from_accept}");
        logln!(
            "服务端视角五元组：本地 {} <-> 远端 {}",
            socket.local_addr()?,
            socket.peer_addr()?
        );
        anyhow::Ok(socket)
    });

    // 客户端主动连接服务端刚刚打印出来的那个地址。
    let client_socket = TcpStream::connect(server_addr)
        .await
        .context("客户端连接失败")?;
    logln!(
        "客户端视角五元组：本地 {} <-> 远端 {}",
        client_socket.local_addr()?,
        client_socket.peer_addr()?
    );

    // 确认服务端也顺利完成了 accept；两边看到的地址应该正好是"镜像"关系：
    // 客户端的（本地, 远端）刚好对应服务端的（远端, 本地）。
    let _server_socket = accept_task.await??;
    logln!("=== 小结：一条 TCP 连接 = 客户端的本地地址 + 服务端的本地地址，二者互为对端 ===");

    Ok(())
}
