# TCP 是可靠字节流

> 代码：`code/network-tcp/`　运行：`cargo run -p network-tcp`

前三课分别讲了分层、地址、socket 编程模型，这一课专门讲 TCP
本身：连接怎么建立、怎么关闭、为什么会“粘包”、为什么有时候
延迟莫名其妙变高、超时应该怎么设。这些不是考试背的知识点，
而是**你线上服务出问题时，日志和现象会直接暴露出来的东西**。

如果还没读过，建议先看 [《socket 就是文件描述符》](socket.md)；
本课会经常和 [《UDP 与双栈 socket》](udp-sockets.md) 做对比。
用户态/内核态与阻塞含义见
[《用户态与内核态》](../os/user-kernel.md)、
[《阻塞与 IO 多路复用》](../os/blocking-io.md)。

----

# 三次握手

> 程序员不需要手动实现握手，但要记住它留下的现象：`connect`/`accept`
> 什么时候返回，返回代表什么。

三次握手的三个包（程序员该记的不是包名，是“谁在等谁”）：

```
客户端                          服务端
  │──── SYN（我要连接） ─────────▶│
  │                              │
  │◀──── SYN-ACK（好，我也确认） ──│
  │                              │
  │──── ACK（收到，开始吧） ──────▶│
  │                              │
  connect() 返回              accept() 返回一条新连接
```

对程序员真正重要的三件事：

1. **`connect` 返回成功，代表三次握手已经完整走完**——不是“发出去了”，
   是“对方也确认收到了”；
2. **`accept` 返回的连接，握手也已经在内核里悄悄完成**，你的代码
   完全不参与这三个包的往来，[《socket 编程模型》](socket.md)
   已经强调过这一点；
3. 如果目标地址没有进程在监听，第一个 SYN 包会得到一个 RST（拒绝）
   响应，`connect` 很快返回“连接被拒绝”错误；如果目标地址完全不通
   （网络层面），SYN 包会被静默丢弃，`connect` 要等到超时才报错——
   **“拒绝”和“超时”是两种不同的失败现象，能帮你判断问题出在
   进程没启动还是网络不通**。

Rust 里区分这两种错误：

```rust
match std::net::TcpStream::connect("127.0.0.1:9999") {
    Ok(_) => {}
    Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
        println!("很快返回：对方机器可达，但没人监听这个端口");
    }
    Err(e) => println!("其他错误（可能是超时）: {e}"),
}
```

对应 Go：

```go
conn, err := net.Dial("tcp", "127.0.0.1:9999")
if err != nil {
    // Go 的错误信息里通常直接写着 "connection refused"
    fmt.Println("连接失败:", err)
}
```

----

# 四次挥手

> 关闭一条连接比建立它多一步，因为 TCP 是双向的，两个方向要分别关闭。

```
主动关闭方                        被动关闭方
  │──── FIN（我这边发完了） ───────▶│
  │◀──── ACK（知道了） ─────────────│
  │                                │
  │◀──── FIN（我这边也发完了） ─────│
  │──── ACK（知道了） ─────────────▶│
  │                                │
  进入 TIME_WAIT                关闭完成
```

四个包对应“双方各自关闭自己的发送方向”：TCP 连接其实是两条独立的
单向字节流叠在一起，`FIN` 只表示“我这个方向不再发了”，不代表
对方那个方向也停了——这一点在下一节“半关闭”会具体展开。

程序员会实际碰到的现象是 **`TIME_WAIT`**：主动发起关闭的那一方，
会在关闭后停留在 `TIME_WAIT` 状态一段时间（常见 60 秒，视系统配置），
才彻底释放这个五元组。这解释了两个常见困惑：

- 为什么重启服务后立刻抢占同一个端口有时会报“地址已被占用”——
  上一批连接可能还有残留的 `TIME_WAIT`，[《UDP 与双栈 socket》](udp-sockets.md)
  用到的 `SO_REUSEADDR` 选项，主要就是为了放宽这个限制；
- 为什么短连接压测时，客户端机器上 `netstat` 会看到大量
  `TIME_WAIT` 连接——这是主动关闭方的正常现象，不是连接泄漏。

> 🔬 底层视角：`TIME_WAIT` 存在的原因，是要保证网络里滞留的旧数据包
> （比如被路由器延迟很久才送达的重复包）不会在同一个五元组被
> 重新使用后，被误认成新连接的数据。这是协议为了正确性做的
> 保守设计，不是实现失误。

----

# 字节流没有消息边界

> 这是 TCP 和 UDP 最本质的区别，也是新手最容易踩的坑：TCP 只保证
> “字节按顺序、完整到达”，不保证“你发一次 `write`，对方就收到
> 完整对应的一次 `read`”。

对比一下 [《UDP 与双栈 socket》](udp-sockets.md) 开头那张表：
UDP 每个数据报天然有边界，TCP 完全没有。这会导致两种现象：

- **粘包**：你连续发了两条消息 `"hello"` 和 `"world"`，对方一次
  `read` 可能同时收到 `"helloworld"`；
- **半包**：你发了一条很长的消息，对方可能要 `read` 好几次才能
  拼出完整内容，也可能一次 `read` 只拿到消息的前半段。

用代码直观感受一下——发送方连续两次 `write`：

```rust
stream.write_all(b"hello")?;
stream.write_all(b"world")?;
```

接收方完全可能只调用一次 `read` 就同时收到两条消息粘在一起：

```rust
let mut buf = [0u8; 1024];
let n = stream.read(&mut buf)?;
// n 可能是 10，buf[..10] 是 "helloworld"，两条消息的边界已经消失了
```

Go 的行为完全一样（这不是某个语言的实现细节，是 TCP 协议本身的性质）：

```go
conn.Write([]byte("hello"))
conn.Write([]byte("world"))
// 另一端 conn.Read(buf) 同样可能一次收到 "helloworld"
```

**应用层必须自己划分消息边界**，常见三种方案：

1. **长度前缀**：每条消息前面先写 4 个字节表示长度，读的时候先读
   长度、再按长度读够字节数（HTTP 的 `Content-Length` 就是这个思路，
   已经在 [《从零手写 HTTP》](../http/http-from-scratch.md) 里用过）；
2. **分隔符**：用固定的分隔符（如 `\r\n`）标记消息结束，读到分隔符
   就认为一条消息完整了（HTTP 的请求行、响应行也是这个思路）；
3. **固定长度**：每条消息都是固定字节数，简单但不灵活，适合协议
   本身消息大小固定的场景。

长度前缀方案的最小实现示意：

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// 发送：先写 4 字节长度（大端），再写消息体
async fn send_framed(stream: &mut tokio::net::TcpStream, msg: &[u8]) -> std::io::Result<()> {
    stream.write_all(&(msg.len() as u32).to_be_bytes()).await?;
    stream.write_all(msg).await
}

// 接收：先读满 4 字节长度，再按长度读满消息体
async fn recv_framed(stream: &mut tokio::net::TcpStream) -> std::io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    let mut msg = vec![0u8; len];
    stream.read_exact(&mut msg).await?;
    Ok(msg)
}
```

对应 Go（`io.ReadFull` 就是专门为“读满指定字节数”设计的）：

```go
func sendFramed(conn net.Conn, msg []byte) error {
    length := make([]byte, 4)
    binary.BigEndian.PutUint32(length, uint32(len(msg)))
    if _, err := conn.Write(length); err != nil {
        return err
    }
    _, err := conn.Write(msg)
    return err
}

func recvFramed(conn net.Conn) ([]byte, error) {
    lengthBuf := make([]byte, 4)
    if _, err := io.ReadFull(conn, lengthBuf); err != nil {
        return nil, err
    }
    length := binary.BigEndian.Uint32(lengthBuf)

    msg := make([]byte, length)
    _, err := io.ReadFull(conn, msg)
    return msg, err
}
```

`read_exact`/`io.ReadFull` 是关键：它们会在内部循环调用底层
`read`，直到真的读满指定字节数（或出错），帮你屏蔽了“一次
`read` 可能读不满”的半包问题。**不要用一次裸的 `read` 就假设
拿到了完整消息**，这是 TCP 编程最常见的 bug 来源。

----

# 半关闭 shutdown

> TCP 连接的两个方向可以分别关闭，这叫“半关闭”，常用于
> “我把该说的话说完了，但还等着收你的回复”这种场景。

`shutdown` 可以只关闭发送方向，不影响接收方向：

```rust
use std::net::Shutdown;

stream.shutdown(Shutdown::Write)?; // 我不再发送了，但还能继续 read
```

对应 Go：

```go
tcpConn := conn.(*net.TCPConn)
tcpConn.CloseWrite() // 只关闭写方向
```

典型场景：客户端把请求体全部发完后，主动 `shutdown(Write)`
告诉服务端“我说完了”，服务端读到对方关闭写方向（`read` 返回 0
字节，即 EOF），知道请求体已经完整，开始处理并把响应写回来，
客户端这边仍然可以正常 `read` 到响应——如果直接完全关闭连接
（而不是半关闭），服务端的响应可能还没写完就被连接关闭打断。

> 🔬 底层视角：完全关闭（`Shutdown::Both` 或直接 drop）会向对方
> 两个方向都发出信号；半关闭只发 `FIN` 影响一个方向，对方的
> `read` 会收到 EOF（返回 0），但对方仍然可以往这个方向的反方向
> 写数据，直到它自己也 `shutdown` 或关闭连接。

----

# Nagle 算法与 TCP_NODELAY

> 默认情况下，TCP 会为了减少小包数量而故意攒一攒再发，这对多数
> 场景是好事，但对“实时性优先”的场景是坏事。

**Nagle 算法** 的逻辑大致是：如果你连续写了几个很小的数据，
TCP 会稍微等一下（等对方确认上一个包，或者等凑够一定大小），
把它们合并成一个包再发出去，减少网络上小包的数量。

这对“吞吐量优先”的场景（比如批量传输大文件）是好事，
但对“每次写入都要求尽快送达”的场景（比如即时聊天、RPC 心跳、
逐帧同步的游戏协议）会带来肉眼可见的延迟。解决办法是关闭 Nagle：

```rust
stream.set_nodelay(true)?;
```

对应 Go：

```go
tcpConn.SetNoDelay(true) // Go 的 net.TCPConn 默认已经是 true（已关闭 Nagle）
```

> 🔬 底层视角：Nagle 算法和另一个叫“延迟确认（delayed ACK）”的
> 机制凑在一起时，可能出现两边互相等待、导致明显卡顿的经典问题——
> 这也是为什么很多要求低延迟的协议库会主动设置 `TCP_NODELAY`，
> 而不是依赖默认行为。**要不要开，取决于你的场景是吞吐优先还是
> 延迟优先，没有万能答案。**

----

# keepalive 探活

> 一条 TCP 连接长时间没有数据往来时，怎么知道对方是不是已经
> 掉线了、只是连接本身还“看起来”存在？

默认情况下，TCP 连接空闲时不会主动发任何数据——如果中间的网络
设备（比如某些云厂商的负载均衡、NAT 网关）在连接空闲一段时间后
悄悄把这条连接的记录清掉，两端的应用程序完全不会察觉，
直到某一方尝试写数据才会发现连接已经“死”了。

**TCP keepalive** 就是解决这个问题的机制：开启后，连接空闲超过
一定时间会自动发探测包，对方正常回应就说明连接还活着，
连续几次探测都没回应就认为连接已经断开。

```rust
use socket2::{SockRef, TcpKeepalive};
use std::time::Duration;

let sock_ref = SockRef::from(&stream);
let keepalive = TcpKeepalive::new().with_time(Duration::from_secs(30));
sock_ref.set_tcp_keepalive(&keepalive)?;
```

对应 Go：

```go
tcpConn.SetKeepAlive(true)
tcpConn.SetKeepAlivePeriod(30 * time.Second)
```

要提醒一句：TCP keepalive 探测的间隔通常是分钟级，**不适合当成
业务层的心跳机制**——如果你的业务需要秒级发现对端异常，应该在
应用层自己实现心跳消息（定期发一条业务心跳、超时没收到就主动
断开重连），而不是依赖 TCP keepalive 的默认参数。

----

# 超时不止一种

> 新手常把“超时”当成一个笼统的概念，实际上至少要分清两种，
> 分别在不同阶段起作用、需要分别设置。

- **连接超时**：`connect` 这一步最多等多久，等太久说明目标机器
  可能不可达或网络中间环节丢包严重；
- **读写超时**：连接已经建立之后，某一次 `read`/`write` 最多等
  多久，等太久可能是对方处理慢、网络拥堵，或者对方已经悄悄断开
  但你还没发现。

Rust 标准库需要手动组合超时（`connect_timeout` 单独存在，
读写超时用 `set_read_timeout`/`set_write_timeout`）：

```rust
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

let addr: SocketAddr = "127.0.0.1:8080".parse()?;
let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3))?;
stream.set_read_timeout(Some(Duration::from_secs(5)))?;
stream.set_write_timeout(Some(Duration::from_secs(5)))?;
```

Tokio 里习惯用 `tokio::time::timeout` 包一层，和
[《UDP 与双栈 socket》](udp-sockets.md) “超时由应用负责”一节
的写法完全一致：

```rust
use tokio::time::{timeout, Duration};

let stream = timeout(
    Duration::from_secs(3),
    tokio::net::TcpStream::connect("127.0.0.1:8080"),
).await.context("连接超时")??;

let n = timeout(Duration::from_secs(5), stream.read(&mut buf))
    .await
    .context("读超时")??;
```

对应 Go（`net.DialTimeout` 管连接，`conn.SetReadDeadline` 管读写）：

```go
conn, err := net.DialTimeout("tcp", "127.0.0.1:8080", 3*time.Second)
conn.SetReadDeadline(time.Now().Add(5 * time.Second))
n, err := conn.Read(buf)
```

**这两种超时必须分开设置**，只设连接超时、不设读写超时的服务，
一旦对方连上之后就不说话（或者说话很慢），会在读操作上无限等待，
慢慢堆积大量“连上了但卡死”的连接，最终耗尽资源。

----

# 背压的直觉

> 为什么有时候你的 `write` 调用也会变慢甚至卡住？TCP 的流量控制
> 在替你做“对方读多慢，我就发多慢”的自动调节。

TCP 连接的发送方和接收方各自维护一个缓冲区。当接收方处理数据
的速度跟不上发送方发送的速度时，接收方的缓冲区会被填满，
TCP 会通过“滑动窗口”机制告诉发送方“先别发那么快”，
发送方的 `write` 调用会因此变慢，甚至阻塞——**这就是背压
（backpressure）在传输层的具体表现：慢的一方，会自动把
“慢”这个信号传导给快的一方，而不需要应用层自己实现流控**。

一个直观的场景：如果接收方是一个正忙着做 CPU 密集计算、
迟迟不去 `read` 的服务，发送方在连续写入足够多数据后，
`write` 会开始变慢甚至阻塞——这不是 bug，是 TCP 在保护
接收方，不让它被数据冲垮。

对比 UDP：UDP 完全没有这套机制，发送方发多快都行，接收方
处理不过来就直接丢包，这也是
[《UDP 与双栈 socket》](udp-sockets.md) 里“别无限 spawn”一节
强调要在应用层自己做流控的原因——**TCP 的背压是协议免费送的，
UDP 没有，得自己实现类似的效果**。

> 🔬 底层视角：TCP 的滑动窗口大小由接收方在每个 ACK 包里携带，
> 动态调整；发送方发出的数据不能超过对方声明的窗口大小。
> 现代操作系统通常还会做“自动窗口调节”，根据实际带宽延迟情况
> 动态放大缓冲区，这也是为什么高延迟高带宽链路上手动调大
> socket 缓冲区有时能明显提升吞吐——细节属于系统调优范畴，
> 这里只需要建立“发送方会被接收方拖慢”的直觉。

----

# Rust 与 Go 的 echo 服务对照

> 把本课讲的握手、粘包、超时串起来，写一个最小可运行的
> echo（原样返回）服务做收尾。

Rust（Tokio）版本：

```rust
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    println!("监听: {}", listener.local_addr()?);

    loop {
        let (mut stream, peer) = listener.accept().await?;
        tokio::spawn(async move {
            let mut buf = [0u8; 1024];
            loop {
                let n = match stream.read(&mut buf).await {
                    Ok(0) => break, // 对方关闭了写方向（EOF）
                    Ok(n) => n,
                    Err(_) => break,
                };
                if stream.write_all(&buf[..n]).await.is_err() {
                    break;
                }
            }
            println!("{peer} 断开");
        });
    }
}
```

Go 版本几乎逐行对应：

```go
func main() {
    listener, _ := net.Listen("tcp", "127.0.0.1:0")
    fmt.Println("监听:", listener.Addr())

    for {
        conn, _ := listener.Accept()
        go func() {
            defer conn.Close()
            buf := make([]byte, 1024)
            for {
                n, err := conn.Read(buf)
                if err != nil { // 包括 io.EOF：对方关闭了写方向
                    break
                }
                if _, err := conn.Write(buf[:n]); err != nil {
                    break
                }
            }
            fmt.Println(conn.RemoteAddr(), "断开")
        }()
    }
}
```

两段代码的核心结构完全一样：`accept` 循环外层，每条连接一个
并发单元（Tokio 任务 / goroutine），连接内部再是一个
`read`→`write` 的小循环，`read` 返回 0（Rust）或
`io.EOF`（Go）都表示对方已经关闭了发送方向。**读到这里，
本课前面每一节讲的现象——握手、EOF、半包、超时——在这段代码里
都能找到对应的位置**。

----

# 常见误区

> 本课前面每一节都提到过一些容易踩的坑，这里单独收集成一份
> 误区清单，排障或写新代码前可以先对照检查一遍。

- **误区一：`connect` 返回成功就代表对方业务处理好了。**
  `connect` 成功只代表**三次握手完成**，双方内核层面已经能
  互相发送字节，和“对方的业务代码有没有准备好处理你的请求”
  是两件完全不同的事——握手是传输层的事，业务是否就绪需要
  应用层自己有一套确认机制（比如先发一个心跳/ready 信号）；
- **误区二：一次 `write` 成功，等价于对方一次 `read` 就能收到
  同样的内容。** “字节流没有消息边界”一节已经详细讲过，粘包
  和半包是 TCP 的正常行为，不是偶发 bug，任何没有做消息边界
  划分的 TCP 代码都存在这个隐患，只是流量小、恰好没触发而已；
- **误区三：`TIME_WAIT` 太多就是连接泄漏，程序有 bug。**
  短连接压测、频繁建立关闭连接的场景下，主动关闭方产生大量
  `TIME_WAIT` 是协议规定的**正常现象**，不代表连接没有被正确
  关闭——真正的连接泄漏应该去看 `ESTABLISHED` 状态的连接数量
  是否只增不减；
- **误区四：只要开了 `TCP_NODELAY`，延迟问题就都解决了。**
  `TCP_NODELAY` 只关闭发送方的 Nagle 算法，接收方那一侧
  “延迟确认（delayed ACK）”仍然可能存在，两边配合才能真正
  避免额外延迟，单独调发送方有时候效果有限；
- **误区五：TCP keepalive 打开了，就不需要应用层心跳了。**
  “keepalive 探活”一节强调过，系统级 keepalive 探测间隔常见是
  分钟级，中间的负载均衡/NAT 网关的空闲超时往往比这个间隔短
  得多，等 TCP keepalive 生效之前，连接可能已经被中间设备清掉；
- **误区六：`accept` 返回之后，我的服务端代码就完全参与了握手。**
  实际情况恰恰相反——`accept` 拿到的连接握手已经在内核里悄悄
  完成，应用代码从头到尾都没有机会、也不需要插手三次握手本身。

----

# 排错对照

> TCP 层面的问题，报错和现象往往比业务日志更早暴露线索，
> 按下面的表格把现象和大概率原因对上号，减少盲猜时间。

| 现象 | 大概率原因 | 排查方向 |
| --- | --- | --- |
| `connect` 几乎立刻失败，报 `ConnectionRefused` | 目标机器可达，但没有进程监听目标端口 | 确认服务是否启动、端口和地址是否配对（详见 [《排障从这几个工具开始》](debug-tools.md)） |
| `connect` 卡很久最后超时 | 网络层不通、中间防火墙丢包，或对方系统防火墙静默拒绝 | 用 `ping` 测网络层可达性，`traceroute`/`tracert` 看中途哪一跳开始异常 |
| `read` 偶尔读到比预期短的数据 | 半包，一次 `read` 没有读满完整消息 | 检查是不是用了裸的 `read` 而不是 `read_exact`/`io.ReadFull`，回顾“字节流没有消息边界”一节 |
| 两条本该独立的消息被拼在一起 | 粘包，没有做长度前缀或分隔符划分 | 检查协议是否有明确的消息边界方案 |
| 客户端 `write` 突然明显变慢 | 接收方处理跟不上，触发了 TCP 背压 | 检查接收方是否有 CPU 密集操作阻塞了 `read` 循环，回顾“背压的直觉”一节 |
| 短连接压测时机器上 `TIME_WAIT` 堆积很多 | 主动关闭方的正常现象，不是泄漏 | 确认 `ESTABLISHED` 连接数是否正常，而不是单看 `TIME_WAIT` 数量 |
| 连接空闲一段时间后，下次写入直接报错 | 中间设备（NAT/LB）把空闲连接清掉了，本地却没察觉 | 参考 [《代理与 NAT》](proxy-nat.md) 的 NAT 转换表机制，配置更短的心跳间隔 |
| 只在生产环境出现连接偶发失败，本地永远正常 | 生产环境有额外的负载均衡/防火墙/安全组策略，本地直连没有这些中间环节 | 对照 [《负载均衡》](load-balancing.md)、[《代理与 NAT》](proxy-nat.md) 排查中间环节配置 |

一条通用建议：**先分清故障发生在“建连接”还是“连接已建立之后”
这两个阶段**——前者去查网络可达性和对方服务状态，后者去查
应用层协议本身（边界划分、超时设置）和中间设备的空闲策略，
两个阶段的排查工具和方向基本不重叠，混在一起查效率会很低。

----

# 动手实验

1. 跑起上面的 echo 服务，用 `telnet 127.0.0.1 端口` 连接后手动敲
   几行字，观察每次敲的内容是否原样返回，体会“交互式”的字节流；
2. 用代码模拟粘包：客户端连续两次 `write` 发送短消息（不加任何
   长度前缀），在服务端打印每次 `read` 实际收到的字节内容，
   观察它们有没有被合并；
3. 把服务端故意 `sleep` 几秒再 `read`，同时让客户端连续写入
   远超默认缓冲区的数据，观察客户端的 `write` 什么时候开始变慢——
   这是背压的直接体验；
4. 分别设置和不设置 `set_nodelay(true)`，用连续发送很多个小消息
   的场景对比延迟差异（可以配合抓包工具观察发送时机）；
5. 用 `netstat -an`（Windows）观察主动关闭连接后残留的
   `TIME_WAIT` 状态，数一数大概停留了多久。

----

# 三句话带走

1. 三次握手在 `connect`/`accept` 返回前已经完成；四次挥手后
   主动关闭方会经历 `TIME_WAIT`，这是协议保证正确性的正常现象。
2. TCP 只保证字节顺序和完整，不保证消息边界——粘包半包必须靠
   长度前缀、分隔符或固定长度在应用层自己划分。
3. 连接超时、读写超时要分开设置；keepalive 探测间隔太粗，
   不能替代应用层心跳；发送方变慢往往是接收方处理不过来触发的
   背压，不是网络故障。

----

# 附：本课生词表

- **三次握手** —— TCP 建立连接的三个包交换过程：SYN、SYN-ACK、ACK。
- **四次挥手** —— TCP 关闭连接的四个包交换过程，两个方向分别关闭。
- **`TIME_WAIT`** —— 主动关闭方在连接关闭后短暂保留的状态，防止旧
  数据包干扰后续连接。
- **粘包 / 半包** —— TCP 无消息边界导致一次读取拿到多条或不完整
  一条消息的现象。
- **长度前缀** —— 在消息前写固定字节数表示消息长度，用于划分边界。
- **半关闭（half-close）** —— 只关闭连接的一个方向（通常是发送方向），
  另一方向仍可用。
- **Nagle 算法** —— 默认会攒小数据合并发送以减少小包数量的机制。
- **`TCP_NODELAY`** —— 关闭 Nagle 算法的 socket 选项，牺牲吞吐换延迟。
- **`SO_KEEPALIVE`** —— 空闲连接定期探活的 socket 选项。
- **背压（backpressure）** —— 接收方处理跟不上时，通过协议机制
  自动拖慢发送方的现象。
- **滑动窗口** —— TCP 用来做流量控制的机制，接收方声明自己还能
  接收多少数据。
- **延迟确认（delayed ACK）** —— 接收方故意延迟发送确认包、
  等凑够条件再回应的机制，和 Nagle 算法凑在一起可能加重延迟。
