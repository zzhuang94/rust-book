//! 最小 TLS echo 演示：用 rcgen 现场生成一张自签名证书，配合 tokio-rustls
//! 在 127.0.0.1 上跑一次真正的 TLS 握手 + 加密数据传输。
//!
//! 跟 network-tcp 课的明文 TCP 相比，区别只有两处：
//!   1. 建立连接之后多了一次 TLS 握手（交换证书、协商密钥）；
//!   2. 握手完成后拿到的 TlsStream 照样实现 AsyncRead/AsyncWrite，
//!      业务代码读写数据的写法跟明文 TCP 完全一样——加解密都是库在背后做的。
//!
//! 运行（在 code/ 下）：cargo run -p network-tls

use std::net::Ipv4Addr;
use std::sync::Arc;

use anyhow::Context;
use labkit::logln;
use rcgen::generate_simple_self_signed;
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, TlsConnector};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logln!("=== 最小 TLS echo 演示 ===");

    // 1) 现场生成一张只对 "127.0.0.1" 这个地址有效的自签名证书。
    //    生产环境的证书由权威 CA 签发；这里是本地教学，自己给自己签一张，
    //    下面第 3 步再让客户端显式信任这张证书（相当于手动装了张根证书）。
    let cert =
        generate_simple_self_signed(["127.0.0.1".to_owned()]).context("生成自签名证书失败")?;
    logln!("已生成自签名证书（仅对 127.0.0.1 有效）");

    // 2) 服务端 TLS 配置：装上刚生成的证书 + 私钥，不要求校验客户端证书
    //    （只做单向 TLS，跟浏览器访问 https 网站的模式一样）。
    let key_der = PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());
    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert.cert.der().clone()], PrivateKeyDer::Pkcs8(key_der))
        .context("构建 TLS 服务端配置失败")?;
    let acceptor = TlsAcceptor::from(Arc::new(server_config));

    // 3) 客户端 TLS 配置：把同一张自签名证书塞进"信任列表"。
    //    真实世界里浏览器信任的是系统/浏览器内置的一堆 CA 根证书；
    //    自签名证书不在这个列表里，所以浏览器会报"不安全"——
    //    这里手动把它加进信任列表，等价于"我自己认这张证书"。
    let mut trusted = RootCertStore::empty();
    trusted
        .add(cert.cert.der().clone())
        .context("把证书加入信任列表失败")?;
    let client_config = ClientConfig::builder()
        .with_root_certificates(trusted)
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(client_config));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("绑定监听地址失败")?;
    let addr = listener.local_addr()?;
    logln!("TLS 服务监听 {addr}（先是普通 TCP，accept 之后才升级成 TLS）");

    let server = tokio::spawn(async move {
        let (tcp, peer) = listener.accept().await?;
        logln!("服务端 accept 到明文 TCP 连接，peer = {peer}");

        // TLS 握手就发生在这一步：交换证书、协商加密套件和会话密钥。
        // 握手成功之后，网络上传输的字节全是加密的，但应用层读到的
        // 还是解密后的原文——TlsStream 把这层复杂度封装掉了。
        let mut tls = acceptor
            .accept(tcp)
            .await
            .context("TLS 握手失败（服务端）")?;
        logln!("服务端 TLS 握手完成");

        let mut buf = [0u8; 128];
        let n = tls.read(&mut buf).await?;
        let msg = String::from_utf8_lossy(&buf[..n]);
        logln!("服务端在 TLS 通道里收到：{msg:?}");

        tls.write_all(b"hello from TLS server\r\n").await?;
        tls.shutdown().await?;
        anyhow::Ok(())
    });

    let tcp = TcpStream::connect(addr).await.context("客户端 TCP 连接失败")?;
    // ServerName 用来校验证书上的"这张证书是给谁用的"跟"我连的是谁"是否匹配，
    // 对应证书生成时写的 subject alt name（这里就是 "127.0.0.1"）。
    let server_name = ServerName::IpAddress(Ipv4Addr::new(127, 0, 0, 1).into());
    let mut tls = connector
        .connect(server_name, tcp)
        .await
        .context("TLS 握手失败（客户端）")?;
    logln!("客户端 TLS 握手完成，开始在加密通道里发送数据");

    tls.write_all(b"hello from TLS client").await?;

    let mut raw = Vec::new();
    tls.read_to_end(&mut raw)
        .await
        .context("读取服务端回复失败")?;
    logln!("客户端收到：{:?}", String::from_utf8_lossy(&raw));

    server.await??;
    Ok(())
}
