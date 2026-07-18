# socket 就是文件描述符

> 代码：`code/network-socket/`　运行：`cargo run -p network-socket`

前两课讲了“分几层”“地址怎么找到对方”，这一课讲程序员真正动手写的那一层：
**socket**。你在 Go 里写 `net.Dial`、在 Rust 里写 `TcpStream::connect`，
它们最终都会落到操作系统提供的 socket API 上。这一课把 socket
到底是什么、客户端和服务端各自的步骤、阻塞与非阻塞的直觉、
几个常见选项，一次讲清楚。

阅读顺序建议：先读 [《分层》](layers.md)、[《寻址》](addressing.md)，
再读这一课；如果“阻塞”“fd”还不熟，先读操作系统组的
[《文件与文件描述符》](../os/file-fd.md) 与
[《阻塞与 IO 多路复用》](../os/blocking-io.md)。

----

# socket 是一个 fd

> 对操作系统来说，一个 socket 和一个打开的文件没什么本质区别——
> 都是一个小整数，指向内核里的一份数据结构。

在 [《从零手写 HTTP》](../http/http-from-scratch.md) 的底层视角里已经提过一次：
**socket 在内核里对应一个文件描述符（fd）**，背后是内核维护的
一对缓冲区（接收缓冲区、发送缓冲区）加上一堆状态信息（连接状态、
本地/远端地址等）。

这解释了几个看起来不直观的现象：

- 为什么 Linux 上 `ulimit -n`（能打开的最大文件数）也限制着
  “最多能开多少个 socket”——因为 socket 真的就算在“文件”这个配额里；
- 为什么 Rust/Go 里关闭连接（`drop`、`Close()`）叫“关闭”而不是
  “断开”——它本质是在告诉内核“这个 fd 我不用了，回收它”；
- 为什么读写 socket 用的函数名和读写文件很像（`read`/`write`）——
  它们在系统调用这一层几乎是同一套接口。

Rust 里可以直接看到这层对应关系（仅 Unix 平台）：

```rust
use std::net::TcpListener;
use std::os::fd::AsRawFd;

let listener = TcpListener::bind("127.0.0.1:0")?;
println!("这个 socket 的 fd 是: {}", listener.as_raw_fd());
```

Go 里也有对应但很少直接用到的方式：

```go
listener, _ := net.Listen("tcp", "127.0.0.1:0")
tcpListener := listener.(*net.TCPListener)
file, _ := tcpListener.File()
fmt.Println("这个 socket 的 fd 是:", file.Fd())
```

日常业务代码几乎不需要直接拿 fd，但知道“socket = fd”这个事实，
能帮你理解很多资源相关的报错（比如 `too many open files`）
到底在说什么。

----

# 五元组唯一标识一条连接

> 内核怎么知道一个收到的数据包该交给哪个 socket？靠五个字段组成的
> “五元组”。

TCP 连接的**五元组**是：

```
(协议, 本地 IP, 本地端口, 远端 IP, 远端端口)
```

举例：`(TCP, 127.0.0.1, 8080, 127.0.0.1, 54321)` 就唯一标识了
一条具体的连接。这解释了一个新手常有的疑问：**同一个端口
（比如服务端的 8080）明明可以同时服务成百上千个客户端连接，
为什么不会互相冲突？**——因为区分它们的不是端口号本身，
而是完整的五元组；只要客户端的 IP 或端口不同，五元组就不同，
就是不同的连接，即使它们都连着服务端的同一个 8080 端口。

对应到 Rust：一个 `TcpListener` 绑定一个本地地址+端口，
`accept()` 之后拿到的每个 `TcpStream` 都携带完整的对端地址，
本质上就是一份五元组：

```rust
let (stream, peer_addr) = listener.accept()?;
let local_addr = stream.local_addr()?;
println!("这条连接的五元组: (TCP, {local_addr}, {peer_addr})");
```

对应 Go：

```go
conn, _ := listener.Accept()
fmt.Println("本地:", conn.LocalAddr(), "远端:", conn.RemoteAddr())
```

> 🔬 底层视角：内核为每条 TCP 连接维护一份独立的状态和缓冲区，
> 用五元组做索引查找。这也是为什么服务端 `accept` 返回的是一个
> **新的** fd（新的 socket），而监听用的那个 fd 继续留着专门等
> 下一个新连接——两者职责不同，不能混用。

----

# 客户端：一步到位

> 客户端只需要知道“我要连去哪”，剩下的交给操作系统。

Rust 客户端建立连接：

```rust
use std::net::TcpStream;

let stream = TcpStream::connect("127.0.0.1:8080")?;
```

对应 Go：

```go
conn, err := net.Dial("tcp", "127.0.0.1:8080")
```

看起来是“一步”，但内核在背后做了：

1. 创建一个新 socket（新 fd）；
2. 让操作系统挑一个本地临时端口（见 [《寻址》](addressing.md)）；
3. 向目标地址发起 TCP 三次握手（细节见 [《TCP》](tcp.md)）；
4. 握手成功后，`connect` 调用返回，你拿到一条可以读写的连接。

如果目标地址没有服务在监听，或者网络不通，`connect` 会在超时后
返回错误——这个超时具体是多久、能不能自己设置，[《TCP》](tcp.md)
的“超时不止一种”一节会细讲。

----

# 服务端：多一步 listen 和 accept

> 服务端比客户端多两个步骤：先“挂牌营业”，再“逐个接客”。

服务端的完整步骤，Rust 版本：

```rust
use std::net::TcpListener;

// 1. socket + bind：占用本地地址和端口
let listener = TcpListener::bind("0.0.0.0:8080")?;

// 2. listen 隐含在 bind 里（Rust 标准库把这两步合并了）；
//    listener 此刻已经在“监听”状态，等待连接到来

// 3. accept：从内核的连接队列里取出一条已经完成三次握手的连接
for incoming in listener.incoming() {
    let stream = incoming?;
    // 4. 每条连接是独立的 TcpStream，可以单独读写
}
```

Go 版本的四步对得更整齐（Go 把 `listen` 显式暴露出来了）：

```go
// 1+2. socket + bind + listen 一起完成
listener, err := net.Listen("tcp", "0.0.0.0:8080")

for {
    // 3. accept：取出一条已完成握手的连接
    conn, err := listener.Accept()
    // 4. conn 是独立的连接，可以单独读写
}
```

理解 `bind`、`listen`、`accept` 三者的分工：

- **bind**：占住“这个地址+端口，只有我能用”；
- **listen**：告诉内核“我准备好接受连接了，请开始排队”，同时指定
  一个排队队列的大小（Go/Rust 标准库通常用一个合理的默认值，
  不需要手动指定）；
- **accept**：从内核已经排好、完成三次握手的连接队列里，取出最前面
  的一条，交给你的代码处理。

一个常被忽略的事实：**`accept` 返回的连接，三次握手已经在内核里
悄悄完成了**，你的代码根本不参与握手过程，`accept` 只是把“已经建好的
连接”这个成果取出来而已。

----

# 阻塞与非阻塞的直觉

> 决定你的代码在“没数据可读”时是干等，还是立刻拿到一个
> “现在还没有”的答案。

默认情况下，socket 的读写是 **阻塞** 的：

```rust
let mut buf = [0u8; 1024];
let n = stream.read(&mut buf)?; // 没数据就一直卡在这一行，直到有数据或出错
```

这行代码执行到一半时，线程被内核挂起（回顾
[《阻塞与 IO 多路复用》](../os/blocking-io.md) 的“阻塞的确切含义”），
直到对方发来数据或连接出错才会继续往下走。**这本身没有错**，
`[《从零手写 HTTP》](../http/http-from-scratch.md)` 的第 0 步就是纯阻塞版本，
简单直接；问题只出现在“一个线程要同时看住很多条连接”的场景。

**非阻塞** 模式下，同样的读操作在没有数据时会立刻返回一个
“现在还不能读”的错误（Unix 上是 `EWOULDBLOCK`/`EAGAIN`），
不会卡住线程：

```rust
use std::io::ErrorKind;

stream.set_nonblocking(true)?;
match stream.read(&mut buf) {
    Ok(n) => { /* 真的读到了 n 字节 */ }
    Err(e) if e.kind() == ErrorKind::WouldBlock => {
        // 现在没数据，稍后再试，线程没有被卡住
    }
    Err(e) => return Err(e.into()),
}
```

对应 Go：Go 的 `net.Conn` 表面上看起来是“阻塞”风格的 `Read`，
但这只是语言层面的假象——Go 运行时（netpoller）在背后把每个连接
悄悄设成了非阻塞模式，`Read` 卡住时挂起的是 **goroutine**，
不是操作系统线程，这正是
[《Go 与 GMP 调度》](../concurrency/go-gmp.md) 讲的核心机制。
Tokio 走的是同一条路：`TcpStream::read().await` 表面顺序风格，
底层也是非阻塞 socket + 事件通知。

> 🔬 底层视角：手写非阻塞代码最麻烦的地方是“现在没数据，
> 那我什么时候该再试一次？”——瞎猜时间会浪费 CPU（轮询太快）
> 或者浪费延迟（轮询太慢）。这正是 [《阻塞与 IO 多路复用》](../os/blocking-io.md)
> “IO 模型三代”里 epoll/Tokio 存在的原因：让内核在数据真正到达时
> 主动通知你，而不是你自己猜。

----

# 常用选项速览

> socket 创建之后，可以通过几个选项微调它的行为。这里先认识名字，
> 具体每个选项什么时候该开，[《TCP》](tcp.md) 会结合现象展开。

[《UDP 与双栈 socket》](udp-sockets.md) 已经实际用过 `socket2` 设置过
两个选项，这里做一个统一整理：

| 选项 | 一句话作用 | 细节见哪里 |
| --- | --- | --- |
| `SO_REUSEADDR` | 放宽本地地址重新绑定的限制 | 本课；udp-sockets.md |
| `SO_REUSEPORT` | 允许多个 socket 绑定同一地址和端口 | udp-sockets.md |
| `TCP_NODELAY` | 关闭 Nagle 算法，小包立即发送 | [《TCP》](tcp.md) |
| `SO_KEEPALIVE` | 开启空闲连接的探活机制 | [《TCP》](tcp.md) |

用 `socket2` 设置选项、再交给 Tokio 管理的完整流程：

```rust
use socket2::{Domain, Socket, Type};

let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;
socket.set_reuse_address(true)?;
socket.bind(&"0.0.0.0:8080".parse::<std::net::SocketAddr>()?.into())?;
socket.listen(128)?;
socket.set_nonblocking(true)?;
let listener = tokio::net::TcpListener::from_std(socket.into())?;
```

对应 Go（通过 `net.ListenConfig` 的 `Control` 回调设置底层选项）：

```go
lc := net.ListenConfig{
    Control: func(network, address string, c syscall.RawConn) error {
        return c.Control(func(fd uintptr) {
            syscall.SetsockoptInt(int(fd), syscall.SOL_SOCKET, syscall.SO_REUSEADDR, 1)
        })
    },
}
listener, err := lc.Listen(context.Background(), "tcp", "0.0.0.0:8080")
```

Go 版本明显更啰嗦——这也是为什么大多数 Go 教程干脆不提这些选项：
`net.Listen` 已经用了合理的默认值，只有少数场景才需要像上面这样
手动下探到系统调用层。

----

# 三套 API 一张表

> Rust 标准库、Tokio、Go 的 `net` 包，对同一件事的叫法几乎一一对应。

| 动作 | Rust `std::net` | Tokio | Go `net` |
| --- | --- | --- | --- |
| 客户端连接 | `TcpStream::connect` | `tokio::net::TcpStream::connect` | `net.Dial` |
| 服务端监听 | `TcpListener::bind` | `tokio::net::TcpListener::bind` | `net.Listen` |
| 接受连接 | `listener.accept()`（阻塞） | `listener.accept().await`（异步） | `listener.Accept()` |
| 读数据 | `stream.read(&mut buf)` | `stream.read(&mut buf).await` | `conn.Read(buf)` |
| 写数据 | `stream.write_all(buf)` | `stream.write_all(buf).await` | `conn.Write(buf)` |
| UDP 收发 | `UdpSocket::recv_from` | `tokio::net::UdpSocket::recv_from().await` | `conn.ReadFromUDP` |

三套 API 名字不同、同步异步不同，但底层映射到的系统调用几乎一样——
这也是为什么读懂了这一课的 socket 模型，切换到任何一套 API
都只是换个函数名而已，模型本身不会变。

----

# 动手实验

1. 在 Rust 里用标准库写一个最小的阻塞 TCP 服务端，`bind` 之后
   直接 `accept`，观察在没有客户端连接时程序卡在哪一行；
2. 把上面的 socket 换成 `set_nonblocking(true)`，用同样的
   `accept` 调用观察它立刻返回 `WouldBlock` 错误，而不是卡住；
3. 同时启动两个客户端连接同一个服务端端口，在服务端打印每条连接的
   `local_addr()`/`peer_addr()`，验证“同一个本地端口，不同的五元组”；
4. 尝试同时用两个进程 `bind` 同一个地址和端口（不设置任何 reuse 选项），
   观察第二个进程报什么错误，再理解 `SO_REUSEADDR` 想解决的问题。

----

# 三句话带走

1. socket 本质是内核里的一个文件描述符（fd），一条 TCP 连接由
   “协议+本地 IP+本地端口+远端 IP+远端端口”五元组唯一标识。
2. 客户端只需要 `connect`；服务端需要 `bind` → `listen` → `accept`
   三步，`accept` 拿到的连接握手已经在内核里完成。
3. 阻塞 socket 没数据时卡住线程，非阻塞 socket 立刻返回“现在不行”；
   Tokio 和 Go 的 goroutine 都是靠非阻塞 socket + 事件通知实现的
   “看起来顺序”的异步模型。

----

# 附：本课生词表

- **fd（文件描述符）** —— 内核用来标识一个打开资源（文件、socket 等）的小整数。
- **五元组** —— `(协议, 本地 IP, 本地端口, 远端 IP, 远端端口)`，唯一标识一条连接。
- **`bind`** —— 让一个 socket 占用指定的本地地址和端口。
- **`listen`** —— 告诉内核这个 socket 准备接受连接，开始排队。
- **`accept`** —— 从已完成握手的连接队列里取出一条连接。
- **阻塞（blocking）** —— 操作没法立即完成时，让当前线程停下来等待。
- **非阻塞（non-blocking）** —— 操作没法立即完成时立刻返回错误，不卡住线程。
- **`WouldBlock`/`EAGAIN`** —— 非阻塞操作“现在不能完成”的标准错误标记。
- **`SO_REUSEADDR`** —— 放宽本地地址重新绑定限制的 socket 选项。
