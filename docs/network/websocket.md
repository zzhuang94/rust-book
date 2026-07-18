# WebSocket 打通双向通道

> 代码：`code/network-websocket/`　运行：`cargo run -p network-websocket`

前面几课讲的都是“一次请求换一次响应”：客户端问一句，服务端答一句，
答完连接可以关掉，也可以留着复用（HTTP keep-alive），但**主动说话的
永远是客户端**。WebSocket 解决的是另一个问题：**服务端也需要随时主动
给客户端发消息**（新消息提醒、实时报价、游戏帧同步），而不是等客户端
下一次来问。

阅读顺序建议：先读 [《TCP 该知道的现象》](tcp.md)（本课的“帧”“心跳”
都建立在 TCP 之上），如果“HTTP 报文长什么样”还不熟，先读
[《从零手写 HTTP》](../http/http-from-scratch.md)；本课末尾会用到的
重连思路，细节留给后续的《超时与重试》（`timeouts-retries.md`）
专门展开。

----

# HTTP Upgrade 完成握手

> WebSocket 连接不是凭空起来的，它先假装是一次普通 HTTP 请求，
> 靠几个特殊 header “请求切换协议”，服务端同意后，这条 TCP 连接
> 就从“HTTP 模式”变成了“WebSocket 帧模式”。

客户端发出的握手请求，本质就是一次带特殊 header 的 GET：

```
GET /chat HTTP/1.1\r\n
Host: example.com\r\n
Upgrade: websocket\r\n
Connection: Upgrade\r\n
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n
Sec-WebSocket-Version: 13\r\n
\r\n
```

服务端如果同意升级，回一个 `101 Switching Protocols`，而不是常见的
`200 OK`：

```
HTTP/1.1 101 Switching Protocols\r\n
Upgrade: websocket\r\n
Connection: Upgrade\r\n
Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n
\r\n
```

`Sec-WebSocket-Accept` 是服务端拿客户端的 `Sec-WebSocket-Key`
做一次固定算法的哈希算出来的——这一步只是为了确认“对方确实懂
WebSocket 协议、不是被浏览器误发的普通 HTTP 请求”，不是加密，
也不是身份认证。**101 响应发出去之后，这条 TCP 连接上再也不会
出现 HTTP 报文，双方开始按“帧”格式通信**，下一节展开。

对应 Rust（`tokio-tungstenite` 把整个升级过程封装成一个函数）：

```rust
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;

let listener = TcpListener::bind("127.0.0.1:9001").await?;
let (stream, _) = listener.accept().await?;
let ws_stream = accept_async(stream).await?; // 内部处理 101 握手
```

对应 Go（`gorilla/websocket` 同样一步到位）：

```go
var upgrader = websocket.Upgrader{}

func handler(w http.ResponseWriter, r *http.Request) {
    conn, err := upgrader.Upgrade(w, r, nil) // 内部处理 101 握手
    if err != nil {
        return
    }
    defer conn.Close()
}
```

两段代码都把“检查 header、算哈希、写 101 响应”这一整套细节隐藏了，
你拿到手的 `ws_stream` / `conn` 已经是一条可以直接读写“帧”的连接。

> 🔬 底层视角：从 TCP 的角度看，握手前后其实是**同一条连接**，
> 三次握手只发生了一次；“协议切换”完全是应用层的约定——TCP
> 根本不知道、也不关心上面跑的是 HTTP 文本还是 WebSocket 帧，
> 这正是 [《分层》](layers.md) 强调的“每层只认自己的信封”。

----

# 帧才是真正载体

> 升级完成之后，双方交换的最小单位叫“帧”（frame），不再是
> HTTP 报文。帧自带类型和长度，天生解决了 TCP 的粘包问题。

一个帧的关键字段（细节字节布局不用背，记住这几个“是什么”）：

| 字段 | 作用 |
| --- | --- |
| FIN | 这是不是一条消息的最后一帧（消息可以拆成多帧发送） |
| opcode | 帧类型：文本（0x1）、二进制（0x2）、关闭（0x8）、ping（0x9）、pong（0xA） |
| mask | 客户端发给服务端的帧必须掩码处理，服务端发给客户端的不需要 |
| payload length | 载荷长度，7 位放不下时用 16 位或 64 位扩展字段 |

**帧自带长度字段**，这一点直接对应 [《TCP》](tcp.md) “字节流没有
消息边界”一节讲的问题：TCP 本身不知道消息边界在哪，需要应用层自己
用长度前缀或分隔符划分；WebSocket 帮你把这件事在协议层做掉了，
你不用再手写 `read_exact` 去拼长度。

Rust 里收发帧，直接是一个枚举类型，不用自己解析字节：

```rust
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

while let Some(msg) = ws_stream.next().await {
    match msg? {
        Message::Text(text) => {
            println!("收到文本帧: {text}");
            ws_stream.send(Message::Text(text)).await?; // 原样回发
        }
        Message::Binary(data) => println!("收到二进制帧: {} 字节", data.len()),
        Message::Close(_) => break,
        _ => {}
    }
}
```

对应 Go：

```go
for {
    msgType, data, err := conn.ReadMessage()
    if err != nil {
        break
    }
    switch msgType {
    case websocket.TextMessage:
        conn.WriteMessage(websocket.TextMessage, data) // 原样回发
    case websocket.BinaryMessage:
        fmt.Printf("收到二进制帧: %d 字节\n", len(data))
    case websocket.CloseMessage:
        return
    }
}
```

两段代码结构完全一致：一个循环，每次拿到“一条完整消息”而不是
“一段不知道边界的字节”——这正是帧格式带来的好处，库已经在内部
把可能分片的多个帧重新拼成一条完整消息再交给你。

----

# 心跳 ping/pong 保活

> WebSocket 协议自带 ping/pong 控制帧，专门用来探活；但和
> [《TCP》](tcp.md) 里的 TCP keepalive 一样，探测间隔怎么定、
> 超时怎么处理，还是要应用层自己拿主意。

协议层的 ping/pong 是两个特殊 opcode 的空（或带少量数据）帧：
一方发 `ping`，对方按协议要求必须尽快回一个携带同样数据的
`pong`。库通常会自动处理“收到 ping 就回 pong”，但主动发 ping、
判断多久没收到 pong 就断线重连，是你自己要写的逻辑。

Rust：

```rust
use tokio::time::{interval, Duration};

let mut ticker = interval(Duration::from_secs(15));
loop {
    tokio::select! {
        _ = ticker.tick() => {
            ws_stream.send(Message::Ping(vec![])).await?;
        }
        msg = ws_stream.next() => {
            if let Some(Ok(Message::Pong(_))) = msg {
                // 收到 pong，重置“上次存活时间”
            }
        }
    }
}
```

对应 Go：

```go
conn.SetPongHandler(func(string) error {
    conn.SetReadDeadline(time.Now().Add(30 * time.Second))
    return nil
})

ticker := time.NewTicker(15 * time.Second)
for range ticker.C {
    conn.WriteMessage(websocket.PingMessage, nil)
}
```

为什么不能只靠 TCP 自带的 keepalive？和 [《TCP》](tcp.md)
“keepalive 探活”一节的结论一样：**系统级 TCP keepalive 的探测
间隔通常是分钟级，而且很多云厂商的负载均衡、反向代理会在自己的
空闲超时时间内直接把连接掐掉，根本等不到 TCP keepalive 起作用**。
WebSocket 协议层的 ping/pong 让你可以把这个周期缩短到秒级，
真正做到“主动、及时地发现对面已经不在了”。

----

# 对比长轮询更省资源

> 在 WebSocket 普及之前，Web 应用实现“服务端推送”常用的办法是
> 长轮询（long polling），理解它的代价，才能明白 WebSocket 省了
> 什么。

三种“让客户端及时知道服务端有新消息”的做法对比：

| 方式 | 客户端行为 | 代价 |
| --- | --- | --- |
| 普通轮询 | 每隔固定时间发一次请求问“有新消息吗” | 大量无意义请求，消息还可能延迟一个轮询周期 |
| 长轮询 | 发请求后服务端“憋着”不回，等有消息才回，回完客户端立刻再发一次 | 每条消息仍要走一次完整 HTTP 请求响应，header 重复开销大 |
| WebSocket | 建一次连接，双方随时可以发帧 | 只握手一次，之后帧头只有几个字节 |

长轮询的关键问题是：**它仍然是“请求—响应”模型在硬撑“服务端主动推送”
的效果**——服务端没法真的主动发消息，只能拖着一个还没回复的请求，
等有事发生了才回复它，回复即算完成一次“推送”，然后客户端必须立刻
发起下一次请求，否则这段时间的推送就会错过。这个模型每条消息都要
重复携带一整套 HTTP header（Cookie、User-Agent 等），而 WebSocket
握手只做一次，后续每帧的额外开销只有几个字节。

> 🔬 底层视角：长轮询本质上仍然复用同一套 HTTP/TCP 连接管理机制，
> 每次“回复+重新发起”之间有个小间隙，如果服务端和客户端都比较忙，
> 消息可能刚好卡在这个间隙里，导致比 WebSocket 多一点延迟——这是
> “用请求响应模拟推送”天然带来的结构性开销，不是某次实现没做好。

----

# 适用场景怎么选

> WebSocket 不是任何时候都比 HTTP 请求好，选型看的是“谁主动、
> 多频繁、要不要双向”。

适合用 WebSocket 的场景，共同特征是**服务端需要主动、频繁地
推送数据给客户端，或者双方都需要随时说话**：

- 聊天、协同编辑：任意一方随时可能发消息，且希望对方尽快看到；
- 实时报价、监控大盘：服务端数据一变就要立刻推给所有在线客户端；
- 游戏帧同步：双向、高频、对延迟敏感。

不适合、或没必要用 WebSocket 的场景：

- 一次性查询（“查一下订单状态”）——普通 HTTP 请求就够了，
  维护一条常驻连接反而是额外成本；
- 大文件上传下载——WebSocket 帧机制不是为这个设计的，直接用
  HTTP 的流式请求/响应体更合适；
- 需要利用 HTTP 缓存、CDN 的场景——WebSocket 是长连接，
  天然绕开了这些基于“请求—响应”的基础设施。

选定用 WebSocket 之后，别忘了**连接会断**——网络抖动、服务端重启、
中间代理超时都可能让连接意外关闭，客户端要有断线重连、带退避的
重试逻辑，这部分留给专门讲重试策略的《超时与重试》
（`timeouts-retries.md`）细讲，本课只强调“一定要设计重连”这个结论。

----

# 代理和负载均衡不一定懂它

> WebSocket 借用了 HTTP 的握手，但握手完成之后的“长连接、
> 双向随时发消息”这套行为，很多传统为普通 HTTP 设计的中间设备
> 并不天然支持好，这是部署阶段最容易踩的坑。

回顾 [《负载均衡》](load-balancing.md) 和 [《代理与 NAT》](proxy-nat.md)
两课的内容，反向代理、负载均衡在 WebSocket 场景下有几个特别
容易翻车的点：

- **代理必须显式支持协议升级转发**：如果反向代理只理解普通
  HTTP 请求响应模型，收到 101 响应后可能直接把连接当成“这次
  请求完事了”给关掉——多数现代反向代理（Nginx 较新版本、云厂商
  LB）都需要专门配置才能正确转发 `Upgrade`/`Connection` 头并
  保持连接不被中途关闭；
- **七层 LB 的空闲超时会误杀正常连接**：普通 HTTP 请求通常几秒
  内就结束，很多 LB 默认的空闲超时设置得很短（比如 60 秒）；
  WebSocket 连接可能长时间没有业务消息往来，如果没有配置更长的
  超时、或者没有做好本课“心跳 ping/pong”一节讲的定期保活，
  连接会被 LB 在业务毫无察觉的情况下悄悄断开；
- **四层 LB 场景下的会话粘滞更重要**：如果同一个 WebSocket
  连接的后续帧被 LB 转发到了不同的后端实例（多发生在没有做好
  连接保持的四层转发配置里），会直接导致协议状态错乱——
  WebSocket 连接一旦建立，整条连接的生命周期都应该固定落在
  同一个后端实例上，回顾 [《负载均衡》](load-balancing.md)
  “会话粘滞是双刃剑”一节，这里粘滞不是可选项，而是必须项。

一条实用建议：**给 WebSocket 服务配置反向代理/LB 时，不要用
默认的“普通 HTTP 反向代理”配置直接套用**，至少要确认三件事：
是否正确转发升级相关的 header、空闲超时是否长于你的心跳周期、
整条连接是否始终路由到同一个后端。

----

# 子协议与压缩选项

> 握手阶段除了确认“要不要升级成 WebSocket”，还可以顺带协商
> 两件常被忽略的事：用什么子协议、要不要压缩。

客户端可以在握手请求里带上 `Sec-WebSocket-Protocol`，列出自己
支持的应用层子协议，服务端从里面选一个确认下来：

```
> Sec-WebSocket-Protocol: chat-v2, chat-v1
< Sec-WebSocket-Protocol: chat-v2
```

这解决的问题是：**WebSocket 帧本身只是“文本”或“二进制”，
帧里面的内容格式（是 JSON？是 protobuf？版本号怎么放？）
完全需要双方另外约定**——子协议字段提供了一个标准化的地方，
让双方在握手阶段就确认清楚，而不是等连接建立之后才发现两边
对帧内容的理解不一致。

压缩方面，常见的 `permessage-deflate` 扩展可以在握手阶段协商
开启，对每条消息做压缩，减少大文本消息（比如较大的 JSON）的
传输体积。开不开、开多大的压缩窗口，同样是在握手阶段的
header 里协商完成，业务代码通常不需要手动处理压缩/解压——
库（`tokio-tungstenite`、`gorilla/websocket`）在设置里打开
选项后就自动处理好了。

----

# 常见误区

> 从普通 HTTP 请求响应模型转过来，容易带错的几个假设。

- **误区一：WebSocket 连接和普通 HTTP 请求一样是“短命”的。**
  WebSocket 连接一旦建立通常会存活很长时间（几分钟到几小时），
  这意味着服务端要为每条连接维护对应的内存状态（比如用户身份、
  订阅的房间），连接数一多，这部分常驻内存开销要提前纳入
  容量规划，不能按“HTTP 请求处理完就释放”的思路估算；
- **误区二：一次 `send` 对应对方一次 `onmessage`，消息不会丢。**
  WebSocket 帧格式解决的是“消息边界”问题，不是可靠性问题——
  它仍然跑在 TCP 上，如果连接本身异常断开，正在传输中的消息
  可能丢失，业务如果需要“消息一定要送达”的保证，要在应用层
  自己加确认机制，WebSocket 协议本身不提供；
- **误区三：ping/pong 就是心跳，不需要业务再做什么。**
  协议层的 ping/pong 只解决“连接是否还活着”，不解决“业务逻辑
  是否还正常”——如果服务端进程卡死在某个业务逻辑死循环里，
  TCP 连接和 WebSocket 层面完全可能仍然“活着”（能正常回 pong），
  业务层面却已经不干活了，这类问题需要额外的业务健康检查；
- **误区四：WebSocket 天生比轮询省资源，所以任何场景都该换。**
  如果客户端数量巨大、但每个连接绝大部分时间都毫无消息往来
  （比如一年才推一次的公告），维护海量长连接本身的服务端内存和
  连接管理开销，可能比“客户端偶尔轮询一下”更贵——选型永远是
  权衡，不是有更“先进”的技术就无脑换。

----

# 排错对照

> WebSocket 握手失败或连接异常断开时，按下面的表格定位问题环节。

| 现象 | 大概率原因 | 怎么确认 |
| --- | --- | --- |
| 握手请求发出去，收到的是 `200` 而不是 `101` | 服务端没有正确处理 `Upgrade` 请求，把它当成了普通 HTTP 请求 | 检查服务端路由是否正确接入了 WebSocket 处理逻辑（如 `accept_async`/`upgrader.Upgrade`） |
| 本地直连正常，经过反向代理/LB 后握手失败 | 代理没有转发 `Upgrade`/`Connection` 头 | 检查代理配置里是否显式声明了支持协议升级 |
| 连接建立后能用一会儿，几十秒到几分钟后必然断开 | 中间的代理/LB/NAT 空闲超时把连接清掉了 | 对照“代理和负载均衡不一定懂它”一节，检查心跳周期是否短于中间设备的空闲超时 |
| 部分帧内容乱码或解析失败 | 双方对帧内容格式理解不一致（没有约定子协议，或版本不一致） | 检查握手阶段是否协商了 `Sec-WebSocket-Protocol`，双方版本是否匹配 |
| 高并发场景下偶尔有连接“串了消息” | 四层 LB 没有保证同一条连接始终转发到同一个后端 | 检查负载均衡的会话粘滞/连接保持配置 |

----

# 动手实验

1. 用浏览器devtools 的 Network 面板打开一个真实使用 WebSocket 的
   网站（很多在线聊天室都用），观察请求列表里那一行 `101` 状态码，
   点开看它的请求/响应 header 里的 `Upgrade`、`Sec-WebSocket-*`；
2. 跑起示例的 echo 服务，用浏览器 console 执行
   `new WebSocket("ws://127.0.0.1:9001").onmessage = e => console.log(e.data)`，
   再手动发几条消息，观察是否原样收到回复；
3. 把示例改成每收到一条文本消息就 `sleep` 2 秒再回复，同时让
   客户端连续发 3 条，观察消息是按顺序逐条处理，还是被“粘”在了一起
   ——对比 [《TCP》](tcp.md) 的粘包现象，体会帧格式已经在库内部
   把边界重新划分好了；
4. 故意在服务端断开连接（不发 `Close` 帧，直接关 TCP），观察客户端
   多久之后才发现连接已经不在了，和主动做 ping/pong 心跳的方案对比
   发现速度的差异。

----

# 三句话带走

1. WebSocket 先靠一次 HTTP Upgrade 握手（101 响应）借用同一条 TCP
   连接，握手完成后双方按“帧”格式通信，不再有 HTTP 报文。
2. 帧自带类型和长度，天生解决了 TCP 的粘包问题；协议层的
   ping/pong 用于探活，但探测周期和断线判断仍要应用层自己实现。
3. 长轮询是用“请求—响应”硬撑推送效果，每条消息都要重复 HTTP
   开销；WebSocket 只握手一次，之后每帧开销很小，更适合双向高频场景。

----

# 附：本课生词表

- **Upgrade** —— HTTP header，用于请求把当前连接切换成另一种协议。
- **101 Switching Protocols** —— 服务端同意协议升级时返回的状态码。
- **帧（frame）** —— WebSocket 通信的最小单位，自带类型和长度字段。
- **opcode** —— 帧头里标识帧类型的字段，如文本、二进制、关闭、ping、pong。
- **掩码（mask）** —— 客户端发往服务端的帧必须对载荷做的一次简单编码。
- **ping/pong** —— WebSocket 协议层内置的探活控制帧，一方发 ping
  对方需回 pong。
- **长轮询（long polling）** —— 服务端拖住请求不立即响应，直到有
  数据才回复的推送模拟方式。
- **`Sec-WebSocket-Protocol`** —— 握手阶段协商应用层子协议（帧
  内容的格式约定）的请求/响应头。
- **`permessage-deflate`** —— WebSocket 常见的按消息压缩扩展，
  握手阶段协商开启。
- **会话粘滞（session affinity）** —— 让同一条连接始终路由到同
  一个后端实例的策略，WebSocket 场景下是必须项而非可选项。
