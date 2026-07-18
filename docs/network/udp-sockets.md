# UDP 与双栈 socket

> 代码：`code/network-udp-sockets/src/main.rs`　运行：  
> `cargo run -p network-udp-sockets`

这一课从零解释一个真实网络服务里很常见、但普通 HTTP 教程不会讲的组合：
**UDP + IPv6 双栈 + 底层 socket 选项**。

先别被 `socket2` 吓住。整段程序只做一件事：服务端收到 `ping`，回一个 `pong`。
我们先弄清 UDP，再把 Rust 标准库、Tokio 和操作系统 socket 接起来。
如果“非阻塞、系统调用、端口”还是生词，先读 [《阻塞与 IO 多路复用》](../os/blocking-io.md)。

建议先读 [《分层是为了解耦》](layers.md) →
[《地址决定连去哪》](addressing.md) →
[《socket 就是文件描述符》](socket.md) →
[《TCP 是可靠字节流》](tcp.md)，再读本课：先弄清分层、寻址、
socket 和 TCP，再对比 UDP 会更顺。学完本课后可继续
[《HTTP 是文本协议》](http-protocol.md) 等应用层章节。

----

# UDP 没有连接

TCP 像打电话：先建立连接，再连续传输字节流。UDP 像寄明信片：每一份数据都独立，
自己带着收件地址。

| TCP | UDP |
| --- | --- |
| 先 `listen`、`accept` | 没有 `accept` |
| 读到的是连续字节流 | 一次收到一个数据报 |
| 保证顺序、重传丢失数据 | 不保证到达、顺序和去重 |
| 应用层要划分消息边界 | 每个数据报天然有边界 |

Tokio 中最关键的两个方法是：

```rust
let (n, peer) = socket.recv_from(&mut buf).await?;
socket.send_to(b"pong", peer).await?;
```

- `recv_from` 返回实际长度 `n` 和发送者地址 `peer`；
- 有效数据只有 `&buf[..n]`，不能把整个缓冲区都当成消息；
- `send_to` 必须明确写目标地址，因为 UDP socket 没有固定连接对端。

对应 Go：

```go
n, peer, err := conn.ReadFromUDP(buf)
_, err = conn.WriteToUDP([]byte("pong"), peer)
```

两边模型几乎一样。Rust 多出来的重点是：`&mut buf` 表示暂时独占借用缓冲区，
`&buf[..n]` 表示只借用本次真正收到的部分。

> 🔩 底层视角：一次 UDP 读取只交付一个数据报。如果数据报大于缓冲区，超出的部分通常会被截断，
> 下一次读取拿不到被截掉的尾巴。协议必须限制包大小，并检查长度。

----

# 地址逐段拆开

示例使用这个监听地址：

```rust
let addr: SocketAddr = "[::]:0".parse()?;
```

逐段看：

- `::` 是 IPv6 的“任意地址”，类似 IPv4 的 `0.0.0.0`；
- IPv6 地址写端口时必须加方括号，所以是 `[::]:0`；
- 端口 `0` 不是实际监听 0 端口，而是请操作系统挑一个空闲端口；
- `SocketAddr` 同时容纳 IPv4 地址和 IPv6 地址。

绑定完成后，用 `local_addr()` 取回操作系统选中的真实端口：

```rust
let server_addr = server_socket.local_addr()?;
```

端口 0 特别适合自动化测试：多个测试并行时，不必大家争抢写死的端口。

注意，`[::]` 是“监听所有本机网卡”的地址，不是客户端应该访问的目标。
示例客户端使用 IPv6 回环地址 `::1`：

```rust
let target = SocketAddr::new("::1".parse()?, server_addr.port());
```

IPv4 的回环地址是 `127.0.0.1`，IPv6 的回环地址是 `::1`。

----

# 双栈是什么

“双栈”表示同一个服务同时面对 IPv4 和 IPv6。示例创建 IPv6 socket 后执行：

```rust
socket.set_only_v6(false)?;
```

`false` 表示这个 socket 不只接 IPv6，也允许操作系统把 IPv4 对端表示成
IPv4-mapped IPv6 地址，例如 `::ffff:127.0.0.1`。

这里有两个必须知道的现实问题：

1. 双栈行为受操作系统配置影响，不能只凭开发机结果猜生产环境；
2. 如果业务按 IP 做白名单、哈希或比较，应先统一地址表示，否则
   `127.0.0.1` 和 `::ffff:127.0.0.1` 可能被误当成两个地址。

Rust 标准库的 `Ipv6Addr::to_ipv4_mapped()` 可以识别 mapped 地址。
做业务判断前，建议把地址归一化成统一形式。

----

# socket2 补底层选项

`tokio::net::UdpSocket::bind` 足够完成普通绑定，但真实服务常常需要在绑定前设置
操作系统选项。`socket2` 提供了更贴近系统调用的接口。

创建过程按顺序分成六步：

```rust
let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?;
socket.set_only_v6(false)?;
socket.set_reuse_address(true)?;
socket.bind(&addr.into())?;
socket.set_nonblocking(true)?;
let socket = UdpSocket::from_std(socket.into())?;
```

每一步分别是：

1. 创建 IPv6、数据报、UDP socket；
2. 开启双栈；
3. 设置地址复用；
4. 绑定本地地址；
5. 切换成非阻塞模式；
6. 把标准库 socket 交给 Tokio 管理。

为什么一定要 `set_nonblocking(true)`？Tokio 的线程不能被某一次普通阻塞读取卡住。
非阻塞 socket 暂时没数据时会立刻告诉运行时“现在还不能读”；Tokio 注册就绪事件，
等操作系统通知可读后，再来推进这个任务。

> 🔩 底层视角：`async` 没有让系统调用凭空变成异步。Tokio 借助 epoll、kqueue 或 IOCP
> 等系统设施等待 socket 就绪，然后唤醒对应 Future。

----

# 复用不要混淆

示例设置了 `SO_REUSEADDR`，Linux 上还设置 `SO_REUSEPORT`：

```rust
socket.set_reuse_address(true)?;

#[cfg(target_os = "linux")]
socket.set_reuse_port(true)?;
```

- `SO_REUSEADDR` 主要放宽本地地址再次绑定的限制；
- `SO_REUSEPORT` 允许多个 socket 绑定相同地址和端口，由内核分发数据；
- 两者语义不同，而且不同操作系统的细节并不完全一致。

`#[cfg(target_os = "linux")]` 表示这一行只在 Linux 编译。
这不是运行时的 `if`：非 Linux 平台编译时，相关代码根本不会进入产物。

不要把 reuse 选项当成“绑定失败万能修复”。生产中是否允许多进程共享端口，
必须由部署模型决定；误开 `SO_REUSEPORT` 可能让数据被另一个进程收到。

----

# 超时由应用负责

UDP 不保证回包。客户端如果无限等待，一次丢包就可能永远挂住。
示例给接收操作套了两秒超时：

```rust
let (n, peer) = timeout(Duration::from_secs(2), client.recv_from(&mut buf))
    .await
    .context("两秒内没收到 UDP 回包")??;
```

这里有两层 `?`：

- 第一层处理 `timeout` 自己的超时错误；
- 第二层处理 `recv_from` 的网络错误。

`.context(...)` 给底层错误补上现场。最终日志不只是“超时”，而是明确告诉你
“哪一步、等了多久、没等到什么”。

服务端也不能默认所有包都合法。最小处理顺序应该是：

1. 检查数据报长度；
2. 解析协议头；
3. 校验版本、类型和字段范围；
4. 无效包记录受限日志后丢弃；
5. 只对合法请求回包。

“受限日志”很重要：公网 UDP 服务如果对每个垃圾包都打印一行日志，攻击者可以先打满磁盘。

----

# 别无限 spawn

最直观的服务器会给每个数据报 `tokio::spawn` 一个任务。但 UDP 没有连接级背压，
流量突然放大时，这会在很短时间内创建海量任务。

常见控制方法有三种：

- 包很轻时，直接在接收循环内处理；
- 固定数量 worker 从有界 channel 取包；
- spawn 前先取得 `Semaphore` 许可，限制并发任务数。

如果 channel 已满，还必须明确策略：丢新包、丢旧包，还是短暂等待。
UDP 本身允许丢包，因此“可观测地丢弃”往往比无限堆内存更安全。

----

# 动手实验

1. 把客户端目标从 `::1` 改成 `127.0.0.1`，观察本机双栈是否接受 IPv4；
2. 把服务端缓冲区从 1024 改成 2，再发送 `ping`，观察消息截断后的行为；
3. 把 `ping` 改成 `hello`，确认服务端不回包、客户端两秒后超时；
4. 注释 `set_nonblocking(true)`，观察 `UdpSocket::from_std` 的报错或异常行为；
5. 连续运行多个实例，体会“随机端口”和“固定端口”的区别。

----

# 三句话带走

1. UDP 一次收发一个数据报，没有连接、顺序保证和自动重传，超时与重试由应用负责。
2. `[::]:0` 表示 IPv6 任意地址加随机端口；双栈还要处理 IPv4-mapped 地址归一化。
3. `socket2` 负责绑定前的底层选项，转交 Tokio 前必须设成非阻塞模式。

----

# 附：本课生词表

- **数据报（datagram）** —— UDP 的独立消息单位，一次发送对应一次接收边界。
- **`SocketAddr`** —— “IP 地址 + 端口”的枚举，可同时表示 IPv4 和 IPv6。
- **双栈（dual stack）** —— 同一服务同时支持 IPv4 与 IPv6。
- **IPv4-mapped IPv6** —— 用 IPv6 形式表达 IPv4 对端的地址，如 `::ffff:127.0.0.1`。
- **`socket2`** —— 暴露底层 socket 创建、选项和绑定能力的 Rust crate。
- **非阻塞（non-blocking）** —— 操作暂时不能完成时立即返回，让异步运行时等待就绪通知。
- **`SO_REUSEADDR`** —— 放宽本地地址复用限制的 socket 选项。
- **`SO_REUSEPORT`** —— 允许多个 socket 共享地址和端口的选项，平台语义有差异。
- **`#[cfg(...)]`** —— 条件编译；条件不满足的代码不会进入编译结果。
