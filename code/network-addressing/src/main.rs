//! 地址解析对比课：IPv4 / IPv6 字符串长什么样，几个常见地址有什么区别，
//! 以及 DNS 解析（主机名 -> IP）最基础的用法。
//!
//! 运行（在 code/ 下）：cargo run -p network-addressing

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};

use labkit::logln;

/// 第 1 步：解析几个常见 IP 字符串，顺便对比几个容易搞混的地址。
fn parse_ip_strings() -> anyhow::Result<()> {
    logln!("—— 第 1 步：解析 IPv4 / IPv6 字符串 ——");

    // 全部脱敏：只用 127.0.0.1（IPv4 回环）、::1（IPv6 回环）、0.0.0.0（通配地址）。
    let loopback_v4: Ipv4Addr = "127.0.0.1".parse()?;
    let loopback_v6: Ipv6Addr = "::1".parse()?;
    let unspecified_v4: Ipv4Addr = "0.0.0.0".parse()?;

    logln!(
        "127.0.0.1 -> {loopback_v4}（是环回地址? {}）",
        loopback_v4.is_loopback()
    );
    logln!(
        "::1       -> {loopback_v6}（是环回地址? {}）",
        loopback_v6.is_loopback()
    );
    logln!(
        "0.0.0.0   -> {unspecified_v4}（是未指定/通配地址? {}）",
        unspecified_v4.is_unspecified()
    );

    logln!("对比小结：");
    logln!("  127.0.0.1 / ::1  = 「只跟本机自己说话」的环回地址（本课示例全程只用它们）。");
    logln!("  0.0.0.0          = 「本机所有网卡地址都收」的通配地址，常见于服务端 bind，不代表某个具体主机。");

    // IpAddr 是 V4/V6 的统一枚举，写通用代码时常用它而不是分别处理两种类型。
    let generic: IpAddr = IpAddr::V4(loopback_v4);
    logln!("统一枚举写法：IpAddr::V4(..) = {generic}");

    Ok(())
}

/// 第 2 步：用 `ToSocketAddrs` 做一次真正的 DNS 解析。
///
/// - "localhost:0" 一定能在本机的名字解析（hosts 文件等）里查到；
/// - "example.com:80" 需要走公网 DNS，脱敏环境或断网情况下可能查不到，
///   查不到不算程序错误，只记录日志说明原因，不 panic、不中断程序。
fn resolve_hostnames() {
    logln!("—— 第 2 步：DNS 解析（主机名 -> IP）——");

    match "localhost:0".to_socket_addrs() {
        Ok(addrs) => {
            for addr in addrs {
                logln!("localhost:0 解析结果 -> {addr}");
            }
        }
        Err(e) => logln!("解析 localhost 失败（本机名字解析异常）：{e}"),
    }

    // example.com:80 走真实公网 DNS；本地/隔离网络下解析失败属于预期情况，仅打日志。
    match "example.com:80".to_socket_addrs() {
        Ok(addrs) => {
            for addr in addrs.take(3) {
                logln!("example.com:80 解析结果（最多显示 3 条）-> {addr}");
            }
        }
        Err(e) => {
            logln!("解析 example.com 失败（可能是当前环境没有外网 DNS，属于预期情况）：{e}");
        }
    }
}

fn main() -> anyhow::Result<()> {
    logln!("=== 地址解析对比课 ===");
    parse_ip_strings()?;
    resolve_hostnames();
    Ok(())
}
