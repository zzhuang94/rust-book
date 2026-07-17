//! TCP echo + "粘包"演示课：TCP 是没有消息边界的字节流，应用层必须自己划界。
//!
//! 这一课分两段演示：
//!   演示 1：故意不做任何划界处理，客户端连续两次 write，服务端用固定缓冲一次
//!          读，展示"粘包"现象——读到的只是一整块字节，看不出消息在哪断开。
//!   演示 2：换成最简单的划界方案——每条消息以 `\n` 结尾，服务端按行读取，
//!          天然获得正确的消息边界，并做一次 echo。
//!
//! 运行（在 code/ 下）：cargo run -p network-tcp

use std::time::Duration;

use anyhow::Context;
use labkit::logln;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

/// 演示 1：TCP 是字节流，没有天然的消息边界（俗称"粘包"）。
async fn demo_sticky_bytes() -> anyhow::Result<()> {
    logln!("—— 演示 1：不做划界，看看什么是「粘包」 ——");

    // 127.0.0.1:0：只用本机环回地址，端口 0 让系统自动分配，脱敏又不冲突。
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("绑定监听地址失败")?;
    let addr = listener.local_addr()?;

    let server = tokio::spawn(async move {
        let (mut socket, _peer) = listener.accept().await?;

        // 故意等一下：给客户端的两次 write 一点时间被操作系统/TCP 合并进
        // 同一次可读事件里，这样更容易稳定复现"一次 read 读到两条消息"的效果。
        tokio::time::sleep(Duration::from_millis(50)).await;

        // 用一个"看起来足够大"的固定缓冲区一次性读，模拟没有做协议划界的服务端。
        let mut buf = [0u8; 1024];
        let n = socket.read(&mut buf).await?;
        let raw = String::from_utf8_lossy(&buf[..n]);

        logln!("服务端一次 read() 收到 {n} 字节，原始内容 = {raw:?}");
        logln!("客户端本来发的是两条独立消息 \"PING\" 和 \"PONGPONG\"，");
        logln!("但服务端看到的只是一整块字节 {raw:?}——");
        logln!("TCP 只保证字节按顺序到达，完全不保证「消息」在哪里断开，这就是「粘包」。");
        logln!("要解决它，必须由应用层自己想办法划界（比如演示 2 的按行读取）。");

        anyhow::Ok(())
    });

    let mut client = TcpStream::connect(addr).await?;
    // 两次独立的 write_all，中间没有任何分隔符，也没有等待对方确认——
    // 这正是"粘包"最典型的成因：应用层发了两条消息，但字节流层面完全连在一起。
    client.write_all(b"PING").await?;
    client.write_all(b"PONGPONG").await?;

    server.await??;
    Ok(())
}

/// 演示 2：最简单的划界方案——每条消息以 `\n` 结尾，按行读写。
async fn demo_line_delimited() -> anyhow::Result<()> {
    logln!("—— 演示 2：简单划界方案——按行 \\n 分隔 ——");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("绑定监听地址失败")?;
    let addr = listener.local_addr()?;

    let server = tokio::spawn(async move {
        let (socket, _peer) = listener.accept().await?;
        let mut reader = BufReader::new(socket);
        let mut line = String::new();

        // read_line 会一直读，直到遇到 \n 为止——即使物理上分好几次 TCP 段到达，
        // 也能天然拼出一条完整消息，这就是"划界"的价值：把边界问题交给协议本身。
        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                logln!("服务端：读到 EOF（对端已关闭连接），退出读取循环");
                break;
            }
            let msg = line.trim_end();
            logln!("服务端按行收到一条完整消息：{msg:?}");

            // echo 回去，末尾也带上 \n，保持协议前后一致。
            let reply = format!("echo:{msg}\n");
            reader.get_mut().write_all(reply.as_bytes()).await?;
        }
        anyhow::Ok(())
    });

    let mut client = TcpStream::connect(addr).await?;
    // 同样是"粘"在一起一次性发出去，但因为每条消息自带 \n 结尾，
    // 读者可以按行精确切分，不再依赖"读了多少字节"这种运气。
    client.write_all(b"HELLO\nWORLD\n").await?;

    {
        // 用一个独立作用域包住可变借用，读完两行回显后就释放，方便后面 shutdown。
        let mut client_reader = BufReader::new(&mut client);
        for _ in 0..2 {
            let mut reply = String::new();
            client_reader.read_line(&mut reply).await?;
            logln!("客户端收到回显：{:?}", reply.trim_end());
        }
    }

    // 主动关闭写端，让服务端的 read_line 读到 EOF（n == 0），从循环里退出。
    client.shutdown().await?;
    server.await??;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logln!("=== TCP 粘包与划界演示 ===");
    demo_sticky_bytes().await?;
    logln!("----------------------------------------");
    demo_line_delimited().await?;
    Ok(())
}
