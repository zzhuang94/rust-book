//! 网络调试工具演示：教你怎么用系统自带的命令行工具"看见"一个正在跑的
//! TCP 服务，而不是只靠代码自己打印的日志。这是排查线上网络问题时最基础
//! 也最常用的手段。
//!
//! 运行（在 code/ 下）：cargo run -p network-debug-tools
//! 程序起来后会打印监听地址，趁它 sleep 的那几秒钟，
//! 可以在另一个终端里试试（把 <port> 换成打印出来的端口号）：
//!   Linux/macOS: ss -tnp | grep <port>      查看这条 TCP 连接的状态
//!   Linux/macOS: lsof -i :<port>            查看哪个进程占用了这个端口
//!   Windows:     netstat -ano | findstr <port>
//!   任意平台：    curl -v http://127.0.0.1:<port>/   触发一次真实连接，
//!                观察三次握手和我们打印出来的 peer 地址

use anyhow::Context;
use labkit::logln;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logln!("=== 网络调试工具演示 ===");

    // 127.0.0.1:0：只监听本机环回地址，端口交给操作系统分配。
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("绑定监听地址失败")?;
    let addr = listener.local_addr()?;
    logln!("监听地址：{addr}");
    logln!("提示：现在可以另开一个终端，用下面的命令观察这个端口——");
    logln!("  Linux/macOS: ss -tnp | grep {}", addr.port());
    logln!("  Linux/macOS: lsof -i :{}", addr.port());
    logln!("  Windows:     netstat -ano | findstr {}", addr.port());
    logln!("  任意平台：    curl -v http://{addr}/   （用来触发一次真实连接）");
    logln!("给你 3 秒钟去开另一个终端手动观察（程序此刻在 sleep，不会退出）...");
    sleep(Duration::from_secs(3)).await;

    // 内置一个客户端自己 connect 一次，模拟"有人访问了这个端口"。
    // 如果你已经在另一个终端手动 curl 过了，下面 accept 到的就是那次连接；
    // 否则 3 秒后没人连过来，这个内置客户端会顶上，保证程序总能往下走。
    let client = tokio::spawn(async move {
        let _ = TcpStream::connect(addr).await;
    });

    let (socket, peer) = listener.accept().await.context("accept 失败")?;
    logln!("accept 到一个连接，peer = {peer}");
    drop(socket);
    logln!("已主动关闭这个连接");

    client.await.ok();
    Ok(())
}
