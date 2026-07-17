use std::net::SocketAddr;

use anyhow::Context;
use labkit::logln;
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};

fn bind_dual_stack(addr: SocketAddr) -> anyhow::Result<UdpSocket> {
    let domain = if addr.is_ipv4() {
        Domain::IPV4
    } else {
        Domain::IPV6
    };
    let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))?;

    if addr.is_ipv6() {
        // false = [::] 同时接 IPv6 和 IPv4-mapped 地址。
        socket.set_only_v6(false)?;
    }
    socket.set_reuse_address(true)?;

    #[cfg(target_os = "linux")]
    socket.set_reuse_port(true)?;

    socket.bind(&addr.into())?;
    socket.set_nonblocking(true)?;

    let std_socket: std::net::UdpSocket = socket.into();
    Ok(UdpSocket::from_std(std_socket)?)
}

async fn server(socket: UdpSocket) -> anyhow::Result<()> {
    let mut buf = [0u8; 1024];

    // UDP 没有 accept：每次 recv_from 直接拿到“一整个数据报 + 对端地址”。
    let (n, peer) = socket.recv_from(&mut buf).await?;
    let data = &buf[..n];
    logln!("服务端收到 {} 字节，来自 {}", n, peer);

    if data.eq_ignore_ascii_case(b"ping") {
        socket.send_to(b"pong\r\n", peer).await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // [::]:0：双栈任意地址；端口 0 让操作系统挑一个空闲端口。
    let addr: SocketAddr = "[::]:0".parse()?;
    let server_socket = bind_dual_stack(addr)?;
    let server_addr = server_socket.local_addr()?;
    logln!("UDP 服务监听 {}", server_addr);

    let task = tokio::spawn(server(server_socket));

    // 测试时用 IPv6 回环访问，避免把 [::] 当成可连接的目标地址。
    let target = SocketAddr::new("::1".parse()?, server_addr.port());
    let client = UdpSocket::bind("[::1]:0").await?;
    client.send_to(b"ping", target).await?;

    let mut buf = [0u8; 64];
    let (n, peer) = timeout(Duration::from_secs(2), client.recv_from(&mut buf))
        .await
        .context("两秒内没收到 UDP 回包")??;
    logln!(
        "客户端收到 {:?}，来自 {}",
        String::from_utf8_lossy(&buf[..n]),
        peer
    );

    task.await??;
    Ok(())
}
