//! 网络分层小故事：一条"应用层消息"是怎么一步步变成"地址"的。
//!
//! 这一课不建立任何真正的连接，只做一件事：
//! 把"我要把消息发给谁"这句大白话，翻译成 Rust 能理解的 `SocketAddr`。
//!
//! 网络分层（从上到下，本课重点是前两层）：
//!   应用层：你的业务消息，比如"给某个服务发一条 ping"
//!   传输层：TCP/UDP，负责端口号（这里体现为 SocketAddr 里的 port）
//!   网络层：IP，负责主机定位（这里体现为 SocketAddr 里的 ip）
//!   链路层：网卡/以太网帧（本课不涉及）
//!
//! 运行（在 code/ 下）：cargo run -p network-layers

use std::net::{SocketAddr, ToSocketAddrs};

use anyhow::Context;
use labkit::logln;

/// 第 1 步：应用层写死几个目标地址字符串，看它们怎么被解析成结构化的 `SocketAddr`。
///
/// 字符串本身对计算机没有意义（只是一串字符），必须先解析（parse）成
/// 结构化的类型，程序才能拿着它去做后续的网络调用。
fn parse_a_few_addresses() -> anyhow::Result<()> {
    logln!("—— 第 1 步：应用层写死几个目标地址字符串 ——");

    // 全部脱敏：只用回环地址 127.0.0.1 / ::1，端口用 0（意为"随便一个空闲端口"）。
    let raw_addrs = ["127.0.0.1:8080", "127.0.0.1:0", "[::1]:9000"];

    for raw in raw_addrs {
        // parse::<SocketAddr>() 要求字符串已经同时带有 IP 和端口，格式必须精确匹配。
        let addr: SocketAddr = raw
            .parse()
            .with_context(|| format!("解析地址失败：{raw}"))?;
        logln!(
            "解析 {raw:>16} -> IP={:<9} 端口={:<6}（是 IPv6? {}）",
            addr.ip(),
            addr.port(),
            addr.is_ipv6()
        );
    }
    Ok(())
}

/// 第 2 步：如果我们只知道主机名（比如 "localhost"），操作系统需要先帮我们
/// 做一次"名字 -> IP"的查询——这就是最基础的 DNS 解析。
///
/// `to_socket_addrs()` 在标准库里是一次"阻塞"调用：它可能触发系统调用去查
/// `/etc/hosts` 或本机的名字解析服务，所以这里不需要 tokio，直接同步调用即可。
fn lookup_localhost() -> anyhow::Result<()> {
    logln!("—— 第 2 步：用 ToSocketAddrs 查 localhost 对应哪些地址 ——");

    // "localhost:0"：端口 0 只是占位，我们真正关心的是 localhost 解析出的 IP 列表。
    let addrs = "localhost:0"
        .to_socket_addrs()
        .context("解析 localhost 失败")?;

    for (i, addr) in addrs.enumerate() {
        logln!(
            "localhost 候选地址[{i}] = {addr}（是 IPv4? {}）",
            addr.is_ipv4()
        );
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    logln!("=== 应用层消息如何变成地址：一个小故事 ===");
    logln!("业务代码想说的话是：「把这条消息发给某个服务」。");
    logln!("但网络协议栈只认地址：IP（网络层负责定位主机）+ 端口（传输层负责定位进程）。");

    parse_a_few_addresses()?;
    lookup_localhost()?;

    logln!("=== 小结：字符串 -> SocketAddr -> （之后才轮到真正发包，见 network-socket 课）===");
    Ok(())
}
