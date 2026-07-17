# MQTT 靠主题做发布订阅

> 代码：`code/network-mqtt/`　运行：`cargo run -p network-mqtt`

[《WebSocket》](websocket.md) 解决的是“一条连接上双方随时能说话”，
但双方仍然是**点对点**——你连的是谁，就只跟谁说话。MQTT 解决的是
另一件事：**发消息的人根本不需要知道谁会收到**，它只管往一个
“主题”里发，谁订阅了这个主题，谁就能收到，发布者和订阅者互相
不认识、也不用建立直接连接。这是它成为 IoT、消息推送场景事实标准
的核心原因。

阅读顺序建议：MQTT 本身跑在 TCP 之上，握手、粘包、心跳这些底层
现象和 [《TCP》](tcp.md) 是同一套；如果还没读过，建议先看那一课。
本课也会和 [《WebSocket》](websocket.md) 反复对比“点对点”与
“发布订阅”两种模型的差异。

----

# 发布订阅解耦生产消费

> MQTT 里有三种角色：发布者（publisher）、订阅者（subscriber）、
> 中间转发的“代理”（broker）。发布者和订阅者永远不直接连接。

三者关系一图说清：

```
发布者 ──publish(topic, payload)──▶  broker  ──推送──▶ 订阅者 A
                                        │
                                        └──推送──▶ 订阅者 B
```

发布者只管往某个主题（topic）发一条消息，完全不知道、也不关心
现在有几个订阅者、订阅者是谁；broker 负责“记住谁订阅了什么主题”，
把每条新消息转发给所有匹配的订阅者。这和你之前写的 TCP echo
服务（[《TCP》](tcp.md) 最后一节）完全不同——echo 服务里，
客户端和服务端是**认识对方、直接通信**的一对连接；MQTT 里，
发布者和订阅者**互不知道对方存在**，中间全靠 broker 转发。

Rust（`rumqttc`）：

```rust
use rumqttc::{Client, MqttOptions, QoS};

let mut options = MqttOptions::new("device-01", "127.0.0.1", 1883);
let (mut client, mut connection) = Client::new(options, 10);

client.subscribe("home/livingroom/temperature", QoS::AtMostOnce)?;
client.publish("home/livingroom/temperature", QoS::AtMostOnce, false, "23.5")?;
```

对应 Go（`paho.mqtt.golang`）：

```go
opts := mqtt.NewClientOptions().AddBroker("tcp://127.0.0.1:1883")
opts.SetClientID("device-01")
client := mqtt.NewClient(opts)
client.Connect().Wait()

client.Subscribe("home/livingroom/temperature", 0, nil)
client.Publish("home/livingroom/temperature", 0, false, "23.5")
```

两段代码里都完全没有出现“对方是谁”，`subscribe`/`Subscribe` 和
`publish`/`Publish` 各自只和 broker 打交道，这就是发布订阅模型
“解耦”的直接体现。

----

# Topic 是分层字符串路由

> Topic 不是一个枚举值，而是像文件路径一样用 `/` 分层的字符串，
> 订阅端还可以用通配符一次订阅一整片。

一个典型 topic 长这样：`home/livingroom/temperature`，
一层一层往下细分。订阅时可以用两种通配符：

| 通配符 | 含义 | 例子 |
| --- | --- | --- |
| `+` | 匹配当前这一层，且仅这一层 | `home/+/temperature` 匹配 `home/livingroom/temperature`，不匹配 `home/a/b/temperature` |
| `#` | 匹配当前层及以下所有层，只能放在末尾 | `home/#` 匹配 `home/livingroom/temperature`、`home/kitchen/humidity` 等所有以 `home/` 开头的主题 |

这一层“字符串路由”，本质上和你在 axum/Gin 里写的 URL 路由是
同一种思路——只是 MQTT 里路由发生在 broker 转发消息时，
而不是 HTTP 服务器分发请求处理函数时。设计 topic 结构时，
建议提前把“谁会用通配符订阅一整片”想清楚，层级顺序一旦定下来，
调整起来牵动所有客户端。

Rust 订阅一整栋楼所有温度传感器：

```rust
client.subscribe("home/+/temperature", QoS::AtMostOnce)?;
```

对应 Go：

```go
client.Subscribe("home/+/temperature", 0, nil)
```

----

# QoS 三档直觉记忆

> MQTT 的 QoS（服务质量等级）不是“网络带宽保证”，而是
> “这条消息允许丢、允许重复到什么程度”的约定，分 0、1、2 三档。

三档的直觉记忆和代价：

| QoS | 直觉 | 保证 | 代价 |
| --- | --- | --- | --- |
| 0 | 发了就不管了 | 最多一次，可能丢 | 最小，一次网络往来 |
| 1 | 发了要对方确认，可能发重 | 至少一次，可能重复 | 多一次 `PUBACK` 确认 |
| 2 | 发了要严格保证只处理一次 | 恰好一次 | 四次交互（`PUBLISH`→`PUBREC`→`PUBREL`→`PUBCOMP`） |

QoS 0 适合“丢了也无所谓、下一条很快又来了”的场景，比如高频上报的
传感器数值；QoS 1 适合“不能丢，但业务能接受偶尔处理两次”的场景
（配合业务层去重）；QoS 2 适合“少量关键指令、绝对不能重复执行”的
场景（比如“关闭阀门”这种指令），但代价是四次交互，吞吐更低。

Rust 指定订阅 QoS：

```rust
client.subscribe("alerts/critical", QoS::ExactlyOnce)?;
```

对应 Go：

```go
client.Subscribe("alerts/critical", 2, nil)
```

> 🔬 底层视角：QoS 保证的是“客户端到 broker”或“broker 到客户端”
> **单跳**之间的可靠性，不是端到端整条链路。如果消息经过多个
> broker 桥接转发（bridge），每一跳各自按自己配置的 QoS 交付，
> 整体端到端的可靠性取决于链路上最弱的一跳，不能想当然认为
> “发布者用了 QoS 2，全链路就绝对不丢不重”。

----

# 会话与遗嘱记录状态

> 客户端断线重连后，broker 要不要记得它之前订阅了什么？客户端
> 意外掉线时，能不能让 broker 帮忙通知别人“它掉线了”？这两个
> 问题分别由“会话”和“遗嘱消息”解决。

**会话（session）**：如果建立连接时声明“非清除会话”（Go/Rust
客户端库通常有对应选项），broker 会记住这个客户端 ID 之前订阅
过什么主题，即使中途断线重连，恢复连接后订阅关系依然有效，
断线期间的消息（如果是 QoS 1/2）也可能被补发。

**遗嘱消息（Last Will and Testament，LWT）**：客户端连接时可以
预先告诉 broker“如果我意外断线（没有正常发送断开请求），
帮我发这条消息到那个主题”。典型用途是设备离线通知：
其他订阅者能第一时间知道某个设备掉线了，而不需要自己去做超时判断。

Rust 设置遗嘱和保活间隔：

```rust
let mut options = MqttOptions::new("device-01", "127.0.0.1", 1883);
options.set_keep_alive(std::time::Duration::from_secs(30));
options.set_last_will(rumqttc::LastWill::new(
    "devices/device-01/status",
    "offline",
    QoS::AtLeastOnce,
    false,
));
```

对应 Go：

```go
opts := mqtt.NewClientOptions().AddBroker("tcp://127.0.0.1:1883")
opts.SetKeepAlive(30 * time.Second)
opts.SetWill("devices/device-01/status", "offline", 1, false)
```

保活间隔（keep alive）决定了客户端要多久发一次 `PINGREQ`——
如果 broker 在一个半保活周期内都没收到任何数据（业务消息或
`PINGREQ` 都算），就认为这个客户端已经掉线，触发遗嘱消息。
这和 [《TCP》](tcp.md) “keepalive 探活”一节的思路一致，
只是 MQTT 把它做成了协议规定的标准行为，而不是要业务自己攒心跳。

----

# 适用 IoT 与消息推送

> MQTT 的设计目标一直很明确：**给带宽小、网络不稳定、设备资源有限
> 的场景，提供一个协议头很轻、能一对多广播的消息通道**。

适合 MQTT 的典型特征：

- 大量设备、每个设备消息量不大——传感器上报、开关状态变化；
- 网络不稳定，需要协议自带断线重连、遗嘱通知的语义；
- 一条消息天然需要广播给多个关心它的人——一个设备状态变化，
  App、后台监控、自动化规则引擎都想同时知道。

不太适合的场景：

- 明确的“一问一答”式调用（“查一下这个订单当前状态并等结果”）——
  MQTT 的发布订阅模型天生不是为这种请求响应设计的，勉强用
  “发请求主题 + 订阅响应主题”模拟会比直接用 RPC 别扭得多，
  这类场景更适合看 [《RPC/gRPC》](rpc-grpc.md)；
- 大文件、大流量传输——MQTT 报文设计偏轻量，不是为大 payload 优化的。

----

# 对照消息服务怎么读

> 这一节不是讲 MQTT 本身，而是给你一份**阅读指引**：本课学的
> “长连接、心跳、主题路由”这些概念，在很多内部长连接消息系统里
> 会用类似的思路实现，只是组件名字不同。下面用完全脱敏的占位名
> 说明典型结构，方便你回去读真实代码时能对上号——**这不是任何
> 生产配置，只是帮你识别代码里各个目录大致在干什么**。

一个常见的长连接消息系统，通常会拆成几个角色（占位名，
不代表任何真实产品）：

- **login**：客户端第一次连接的入口，通常只做“认证 + 分配一个
  可用的接入地址”，不负责长期保持连接；
- **linker**：客户端真正长期保持连接、收发消息的节点——建好连接后，
  客户端的心跳、上行消息、下行推送都走这条连接；
- **dispatch / broker**：负责“消息该发给谁”的路由决策，
  可能维护着“某个用户/某个主题当前连在哪个 linker 上”这类映射，
  收到一条消息后决定转发到哪个 linker，再由 linker 推给具体客户端。

用一句话概括整体流程：**客户端先连 login 拿到一个入口地址，
再拿着这个地址去连 linker 保持长连接、收发消息，dispatch/broker
在背后做路由决策**——这和本课“发布者/订阅者互不认识，
中间靠 broker 转发”的思路是同一类结构，只是多了一步“先找入口”。

这几个概念在你本课学的内容里都能找到对应：

| 本课概念 | 对照 |
| --- | --- |
| 长连接 | 客户端连上 linker 之后，这条连接会一直保持，和 MQTT 客户端连 broker 后维持的连接是同一类东西 |
| 心跳 | linker 上的心跳判断连接是否存活，思路等同 MQTT 的 `PINGREQ`/`PINGRESP` 和保活间隔 |
| 主题/路由 | dispatch/broker 做的路由决策，对应 MQTT 里 broker 按 topic 转发给订阅者的过程 |
| 序列化边界 | linker 收发的每条消息也需要有明确的长度/边界划分，思路和 [《TCP》](tcp.md) 的长度前缀、MQTT 报文自带的剩余长度字段是同一个问题 |

如果你手头有类似结构的代码仓库，建议**自己在本地打开目录读一读**，
重点看这几件事：`login` 目录下是不是真的只处理“认证 + 分配地址”
就结束，不维护长连接；`linker` 目录下的心跳判断逻辑是怎么写的
（多久没收到心跳算掉线）；`dispatch` 或 `broker` 目录下，
消息路由的映射关系是存在内存里还是外部存储里。这份对照只是
给你一个“先看哪、大致找什么”的地图，具体到某个仓库的真实实现、
地址、端口、鉴权方式，都以你本地代码和文档为准，本课不展开、
也不涉及任何生产环境的具体配置。

----

# 动手实验

1. 本地跑一个开源 MQTT broker（例如某些语言自带的开发用 broker，
   监听 `127.0.0.1:1883`），用示例代码的发布者和订阅者各连一次，
   验证发布订阅的基本流程；
2. 用 `+` 通配符订阅 `home/+/temperature`，分别往
   `home/livingroom/temperature` 和 `home/kitchen/humidity` 发消息，
   确认只有前者能被收到；
3. 让订阅者故意断开重连（不设置清除会话），观察断线期间发布的
   QoS 1 消息，重连后是否被补发；
4. 给客户端设置遗嘱消息后，直接 kill 掉客户端进程（不走正常断开
   流程），观察 broker 是否把遗嘱消息发给了其他订阅者；
5. 分别用 QoS 0 和 QoS 2 发送同一批消息，用抓包工具数一数
   两种方式各自产生了几次网络往来。

----

# 三句话带走

1. MQTT 是发布订阅模型：发布者和订阅者互不知道对方存在，
   全靠 broker 按 topic 转发，这和点对点的 TCP/WebSocket 连接
   是完全不同的思路。
2. QoS 0/1/2 约定的是“单跳之间”消息可能丢、可能重、还是恰好一次，
   不是端到端整条链路的保证；会话和遗嘱消息分别解决“断线重连
   要不要记得订阅”和“意外掉线要不要通知别人”。
3. 阅读真实长连接消息系统时，可以按“先连入口（login）、
   再连长连接节点（linker）、路由靠 dispatch/broker”这个通用结构
   去定位代码，但这只是阅读指引，不代表任何具体生产配置。

----

# 附：本课生词表

- **发布订阅（pub/sub）** —— 发布者和订阅者不直接通信，全靠中间
  broker 按主题转发的通信模型。
- **broker** —— 负责接收发布消息并转发给匹配订阅者的中间角色。
- **topic** —— 用 `/` 分层的字符串，消息的分类标识，也是订阅的
  匹配对象。
- **通配符 `+` / `#`** —— 订阅主题时用于匹配单层 / 匹配当前层及
  以下所有层的特殊符号。
- **QoS（服务质量等级）** —— MQTT 里约定消息交付可靠程度的三档：
  最多一次、至少一次、恰好一次。
- **会话（session）** —— broker 为某个客户端 ID 记住的订阅关系和
  未确认消息状态。
- **遗嘱消息（LWT）** —— 客户端预先登记、在其意外掉线时由 broker
  代为发布的消息。
- **保活间隔（keep alive）** —— 客户端承诺的最长静默时间，超过则
  broker 认为其已掉线。
