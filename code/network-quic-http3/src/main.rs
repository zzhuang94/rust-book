//! QUIC / HTTP3 直觉教学程序。
//!
//! 默认（不开任何 feature）只做一件事：用一个纯 UDP 的小演示，对比
//! TCP"粘包"（详见 network-tcp 课）——TCP 是没有消息边界的字节流，
//! UDP 每次收发天然是"一整条独立消息"。这正是 QUIC 选择 UDP 打底、
//! 并按"流"分别维护可靠性的直觉起点：**一条流的丢包不该拖累别的流**。
//!
//! 如果打开 `quic-demo` feature（依赖 `quinn`，编译比默认重不少），
//! 额外跑一次真正的 QUIC 最小 echo，用真实握手 + 流收发验证上面的直觉。
//!
//! 配套文档：docs/network/quic-http3.md
//! 运行（在 code/ 下）：cargo run -p network-quic-http3
//! 带真实 QUIC 演示：cargo run -p network-quic-http3 --features quic-demo

use std::time::Duration;

use anyhow::Context;
use labkit::logln;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::time::timeout;

/// 演示 1（复习）：TCP 是字节流，没有天然的消息边界。
///
/// 完整版本、更细的解释见 `network-tcp` 课；这里只留一个最短复现，
/// 方便紧接着和演示 2 的 UDP 结果直接对照。
async fn tcp_sticky_recap() -> anyhow::Result<()> {
    logln!("—— 复习：TCP 是字节流，两次 write 可能被读成一整块 ——");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("绑定 TCP 监听失败")?;
    let addr = listener.local_addr()?;

    let server = tokio::spawn(async move {
        let (mut socket, _peer) = listener.accept().await?;
        // 故意等一下，让两次 write 更容易被合并进同一次可读事件。
        tokio::time::sleep(Duration::from_millis(30)).await;
        let mut buf = [0u8; 1024];
        let n = socket.read(&mut buf).await?;
        anyhow::Ok(String::from_utf8_lossy(&buf[..n]).into_owned())
    });

    let mut client = TcpStream::connect(addr).await?;
    client.write_all(b"MSG-A").await?;
    client.write_all(b"MSG-B").await?;

    let raw = server.await??;
    logln!("TCP 服务端一次 read() 收到：{raw:?}——两条消息的边界已经看不出来了");
    Ok(())
}

/// 演示 2：UDP 每次收发是一整条独立数据报，天然带消息边界。
///
/// 关键对照：即使客户端连续发了 3 条消息，服务端也必须 `recv_from` 3 次，
/// 每次刚好拿到一整条、边界清晰的消息——这和 TCP"一次 read 可能读到
/// 几条消息粘在一起"完全不同。QUIC 在 UDP 之上按"流"组织数据，
/// 继承的正是这种"各自独立、互不拖累"的特性，只是在流内部又自己
/// 加了一层可靠性和顺序保证（UDP 本身不保证到达、不保证顺序）。
async fn udp_independent_messages_demo() -> anyhow::Result<()> {
    logln!("—— 演示：UDP 天然带消息边界，一次 recv_from = 一整条消息 ——");

    let server = UdpSocket::bind("127.0.0.1:0")
        .await
        .context("绑定 UDP 监听失败")?;
    let server_addr = server.local_addr()?;

    let client = UdpSocket::bind("127.0.0.1:0")
        .await
        .context("绑定 UDP 客户端失败")?;

    let messages = ["流A-第1条", "流B-第1条", "流A-第2条"];
    for msg in &messages {
        client.send_to(msg.as_bytes(), server_addr).await?;
        // 稍微错开发送时间，方便和"QUIC 按流隔离"的叙事对上号：
        // 就算把这 3 条消息想象成属于两个不同的"流"，它们在 UDP 层面
        // 本来就是互相独立的数据报，谁先到、谁丢了，都不会粘连别人。
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    let mut buf = [0u8; 128];
    for i in 0..messages.len() {
        let (n, peer) = timeout(Duration::from_secs(1), server.recv_from(&mut buf))
            .await
            .context("等待 UDP 数据报超时")?
            .context("recv_from 失败")?;
        logln!(
            "UDP 服务端第 {} 次 recv_from 收到完整消息 {:?}，来自 {peer}",
            i + 1,
            String::from_utf8_lossy(&buf[..n])
        );
    }

    logln!("对照结论：3 条消息，recv_from 恰好调用 3 次，边界由 UDP 自己维护，");
    logln!("应用层完全不用像 TCP 那样自己拼 \\n 或长度前缀来切分消息。");
    Ok(())
}

/// 用注释讲清楚"如果这是 QUIC，丢包会怎样"——这里不做真的丢包模拟
/// （UDP/quinn 都不方便在教学代码里可控地丢包），只把直觉写清楚。
fn explain_quic_head_of_line_isolation() {
    logln!("—— QUIC 直觉：把上面的 3 条 UDP 消息想象成两条 QUIC 流 ——");
    logln!("假设「流A-第1条」和「流A-第2条」属于流 A，「流B-第1条」属于流 B：");
    logln!("  · 如果流 A 的第 1 条在网络上丢了，QUIC 只会让流 A 等待重传，");
    logln!("    流 B 的数据完全不受影响，应用层能立刻拿到流 B 的内容；");
    logln!("  · 放到 TCP+HTTP/2 里，这 3 条消息会被拼进同一条 TCP 字节流，");
    logln!("    丢其中任何一段都会让整条连接的后续数据全部卡住等重传——");
    logln!("    这就是上一节文档讲的「队头阻塞」，QUIC 的分流设计正是为了避开它。");
}

#[cfg(feature = "quic-demo")]
mod quic_demo {
    use std::net::SocketAddr;
    use std::sync::Arc;

    use anyhow::Context;
    use labkit::logln;
    use quinn::crypto::rustls::{QuicClientConfig, QuicServerConfig};
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    use rustls::{ClientConfig, RootCertStore, ServerConfig};

    const ALPN: &[u8] = b"network-quic-http3-demo";

    /// 服务端：accept 一条连接、accept 一条双向流，读到内容后原样回写。
    async fn run_server(endpoint: quinn::Endpoint) -> anyhow::Result<()> {
        let incoming = endpoint
            .accept()
            .await
            .context("QUIC 服务端没有等到任何连接")?;
        let conn = incoming.await.context("QUIC 握手失败（服务端）")?;
        logln!("[QUIC 服务端] 新连接来自 {}", conn.remote_address());

        let (mut send, mut recv) = conn.accept_bi().await.context("接受双向流失败")?;
        let data = recv.read_to_end(1024).await.context("读取流数据失败")?;
        logln!(
            "[QUIC 服务端] 收到流数据：{:?}",
            String::from_utf8_lossy(&data)
        );

        send.write_all(&data).await.context("回写流数据失败")?;
        send.finish().context("结束发送流失败")?;

        // 重要：`conn` 一旦被 drop 就会隐式发起关闭（close code 0），
        // 如果客户端还没读完回显就发生这次隐式关闭，会被客户端读到
        // "connection lost / closed by peer" 的报错。这里等客户端
        // 读完数据后主动 `conn.close(..)`，服务端等到那个时刻才退出，
        // 避免"我这边发完了就立刻挂电话"抢在对方听完之前关闭连接。
        conn.closed().await;
        Ok(())
    }

    /// 客户端：连服务端、开一条双向流、发数据、读回显。
    async fn run_client(
        server_addr: SocketAddr,
        client_config: quinn::ClientConfig,
    ) -> anyhow::Result<()> {
        let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse()?)
            .context("创建 QUIC 客户端 endpoint 失败")?;
        endpoint.set_default_client_config(client_config);

        let conn = endpoint
            .connect(server_addr, "localhost")
            .context("发起 QUIC 连接失败")?
            .await
            .context("QUIC 握手失败（客户端）")?;
        logln!("[QUIC 客户端] 已连接到 {}", conn.remote_address());

        let (mut send, mut recv) = conn.open_bi().await.context("打开双向流失败")?;
        send.write_all(b"hello quic")
            .await
            .context("发送数据失败")?;
        send.finish().context("结束发送流失败")?;

        let reply = recv.read_to_end(1024).await.context("读取回显失败")?;
        logln!(
            "[QUIC 客户端] 收到回显：{:?}",
            String::from_utf8_lossy(&reply)
        );

        conn.close(0u32.into(), b"demo done");
        endpoint.wait_idle().await;
        Ok(())
    }

    /// 真正跑一次 QUIC echo：现场生成自签名证书，服务端/客户端各建一个
    /// endpoint，验证本课"直觉部分"讲的握手、流收发是真实可运行的。
    pub async fn run() -> anyhow::Result<()> {
        logln!("—— 额外演示：真正的 QUIC 最小 echo（quic-demo feature）——");

        let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
            .context("生成自签名证书失败")?;
        let cert_der: CertificateDer = cert.cert.der().clone();
        let key_der = PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());

        let mut server_crypto = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der.clone()], PrivateKeyDer::Pkcs8(key_der))
            .context("构建 QUIC 服务端 TLS 配置失败")?;
        server_crypto.alpn_protocols = vec![ALPN.to_vec()];
        let server_config = quinn::ServerConfig::with_crypto(Arc::new(
            QuicServerConfig::try_from(server_crypto).context("转换 QUIC 服务端配置失败")?,
        ));

        let endpoint = quinn::Endpoint::server(server_config, "127.0.0.1:0".parse()?)
            .context("绑定 QUIC 服务端失败")?;
        let server_addr = endpoint.local_addr()?;
        logln!("[QUIC 服务端] 监听 {server_addr}");

        let mut trusted = RootCertStore::empty();
        trusted.add(cert_der).context("把证书加入信任列表失败")?;
        let mut client_crypto = ClientConfig::builder()
            .with_root_certificates(trusted)
            .with_no_client_auth();
        client_crypto.alpn_protocols = vec![ALPN.to_vec()];
        let client_config = quinn::ClientConfig::new(Arc::new(
            QuicClientConfig::try_from(client_crypto).context("转换 QUIC 客户端配置失败")?,
        ));

        let server_task = tokio::spawn(run_server(endpoint));
        run_client(server_addr, client_config).await?;
        server_task.await??;

        logln!("QUIC echo 演示结束：握手、开流、收发、关闭全部走了一遍真实协议。");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logln!("=== QUIC / HTTP3 直觉教学程序 ===");

    tcp_sticky_recap().await?;
    logln!("----------------------------------------");
    udp_independent_messages_demo().await?;
    logln!("----------------------------------------");
    explain_quic_head_of_line_isolation();

    #[cfg(feature = "quic-demo")]
    {
        logln!("----------------------------------------");
        quic_demo::run().await?;
    }

    #[cfg(not(feature = "quic-demo"))]
    {
        logln!("----------------------------------------");
        logln!("提示：加 --features quic-demo 可以额外跑一次真正的 QUIC 最小 echo。");
    }

    Ok(())
}
