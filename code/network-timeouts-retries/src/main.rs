//! 超时与重试演示：网络请求会卡在三种地方——连不上（connect）、
//! 连上了但对方半天不说话（read）、以及"偶尔失败一下没关系，重试就好"。
//! 这一课分三段演示，全部只用 127.0.0.1，不依赖任何外部网络。
//!
//! 运行（在 code/ 下）：cargo run -p network-timeouts-retries

use std::future::pending;
use std::time::Duration;

use anyhow::Context;
use labkit::logln;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{sleep, timeout};

/// 演示 1：connect 超时。
///
/// 真实业务里，"connect 超时"常用于连接一个网络不可达、或被防火墙悄悄丢弹
/// SYN 包的"黑洞"地址——三次握手的第一步发出去后没有任何回应，连接会一直
/// 卡在"连接中"，直到我们自己设的超时把它打断。
///
/// 但本教程约定只能用 127.0.0.1（本机环回），环回接口没有真正的网络延迟：
/// 连一个没人监听的端口，操作系统会立刻回一个 RST，`connect()` 立刻返回
/// "连接被拒绝"的错误，根本等不到超时触发——这是新手很容易搞混的一点，
/// 下面先用真实的本机行为验证它。
async fn demo_connect_timeout() -> anyhow::Result<()> {
    logln!("—— 演示 1：connect 超时 ——");

    // 1) 本机真实情况：先绑定拿到一个端口号，随即释放监听者，
    //    这个端口就变回"无人监听"状态，模拟"服务没起来"。
    let closed_addr = {
        let probe = TcpListener::bind("127.0.0.1:0").await?;
        probe.local_addr()?
        // probe 在这里被 drop，端口立刻恢复成无人监听。
    };
    match timeout(Duration::from_millis(300), TcpStream::connect(closed_addr)).await {
        Ok(Ok(_)) => logln!("意外：居然连上了 {closed_addr}"),
        Ok(Err(e)) => {
            logln!("连接「无人监听」的 {closed_addr} 立刻收到错误：{e}（是拒绝，不是超时）")
        }
        Err(_) => logln!("居然等到了超时——本机环回上这种情况很罕见"),
    }

    // 2) 模拟"黑洞地址导致 connect 悬着不动"：用一个永远不会 resolve 的
    //    future 占位。换成真正的黑洞地址，业务代码写法和这里一模一样，
    //    都是"把可能挂起的 IO 操作用 timeout 包一层"。
    let hang_forever = pending::<std::io::Result<TcpStream>>();
    match timeout(Duration::from_millis(300), hang_forever).await {
        Ok(_) => unreachable!("pending() 永远不会 resolve"),
        Err(_) => logln!("模拟黑洞地址：300ms 内没连上，超时打断——这才是真正的「connect 超时」"),
    }

    Ok(())
}

/// 演示 2：read 超时。服务端故意接了连接却"慢半天不回话"，
/// 客户端不能傻等，得给 read 也设个超时。
async fn demo_read_timeout() -> anyhow::Result<()> {
    logln!("—— 演示 2：read 超时（服务端故意“慢” ）——");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("绑定监听地址失败")?;
    let addr = listener.local_addr()?;

    let server = tokio::spawn(async move {
        let (mut socket, peer) = listener.accept().await?;
        logln!("慢服务端：accept 到 {peer}，故意 sleep 800ms 才回复");
        sleep(Duration::from_millis(800)).await;
        socket.write_all(b"finally...\r\n").await?;
        anyhow::Ok(())
    });

    let mut client = TcpStream::connect(addr).await.context("客户端连接失败")?;
    let mut buf = [0u8; 64];
    match timeout(Duration::from_millis(200), client.read(&mut buf)).await {
        Ok(Ok(n)) => logln!("居然在 200ms 内就读到了 {n} 字节"),
        Ok(Err(e)) => logln!("read 出错：{e}"),
        Err(_) => {
            logln!("200ms 内没读到任何数据，read 超时——正确做法是断开重试，而不是死等对方")
        }
    }

    // 服务端还在慢慢跑，等它跑完避免留下悬空任务；生产代码里这里通常是
    // 直接 drop 连接 + 触发重试，不会真的等对方跑完。
    server.await??;
    Ok(())
}

/// 演示 3：简单重试循环 + 指数退避。服务"还没准备好"时，
/// 一次性失败没必要放弃，稍等一下再试，往往就成了。
async fn demo_retry_with_backoff() -> anyhow::Result<()> {
    logln!("—— 演示 3：重试 + 退避 ——");

    // 先探测一个空闲端口号，然后马上释放，让"服务还没启动"这件事变得真实：
    // 此刻这个端口上确确实实没有任何东西在监听。
    let addr = {
        let probe = TcpListener::bind("127.0.0.1:0").await?;
        probe.local_addr()?
    };
    logln!("服务将会监听 {addr}，但故意晚一点才真正 bind + accept");

    let server = tokio::spawn(async move {
        // 模拟"服务启动慢"：先睡一会儿，前几次客户端的连接尝试注定失败。
        sleep(Duration::from_millis(650)).await;
        let listener = TcpListener::bind(addr).await?;
        logln!("服务端终于就位，开始监听 {addr}");
        let (_socket, peer) = listener.accept().await?;
        logln!("服务端 accept 到 {peer}");
        anyhow::Ok(())
    });

    let mut delay = Duration::from_millis(100);
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        logln!("第 {attempt} 次尝试连接 {addr} ...");
        match TcpStream::connect(addr).await {
            Ok(_stream) => {
                logln!("第 {attempt} 次尝试成功！");
                break;
            }
            Err(e) => {
                logln!("第 {attempt} 次失败：{e}，退避 {delay:?} 后重试");
                sleep(delay).await;
                // 指数退避，翻倍增长但设个上限，避免退避时间无限膨胀。
                delay = (delay * 2).min(Duration::from_secs(1));
            }
        }
    }

    server.await??;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logln!("=== 超时与重试演示 ===");
    demo_connect_timeout().await?;
    logln!("----------------------------------------");
    demo_read_timeout().await?;
    logln!("----------------------------------------");
    demo_retry_with_backoff().await?;
    Ok(())
}
