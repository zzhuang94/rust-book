# RPC 让远程调用像本地

> 代码：`code/network-rpc-grpc/`　运行：`cargo run -p network-rpc-grpc`

[《MQTT》](mqtt.md) 讲的是“发布者根本不知道谁在听”，这一课讲的是
另一个极端：**明确知道要调用谁、调用哪个方法、传什么参数、等什么
返回值**——这正是 RPC（远程过程调用）的核心追求：**让调用一个
远程服务，长得和调用一个本地函数一样**。本课以 gRPC 为主线，
它是目前最常见的 RPC 实现之一，用 protobuf 定义契约、跑在
HTTP/2 之上。

阅读顺序建议：如果还没读过 [《TCP》](tcp.md) 和
[《从零手写 HTTP》](../http/http-from-scratch.md)，建议先读——
gRPC 底层跑在 HTTP/2 上，理解“请求—响应”和“流式”的区别需要先有
HTTP 的基础;超时和重试的通用原则见后续《超时与重试》
（`timeouts-retries.md`）。

----

# RPC 与 REST 两种直觉

> REST 站在“资源”的角度思考：URL 代表一个资源，方法（GET/POST/
> PUT/DELETE）代表对这个资源做什么；RPC 站在“动作”的角度思考：
> 直接调用一个命名好的方法，像调用本地函数一样。

同一件事“查询用户信息”，两种风格写出来完全不同：

REST 风格：

```
GET /users/123 HTTP/1.1
```

RPC 风格（gRPC）：

```rust
let response = client.get_user(GetUserRequest { id: 123 }).await?;
```

两种风格的直觉差异：

| | REST | RPC（gRPC） |
| --- | --- | --- |
| 思考角度 | 资源 + 动作 | 方法调用 |
| 契约来源 | 通常靠文档约定（OpenAPI 可选） | IDL（`.proto`）强制约定，代码生成 |
| 数据格式 | 通常 JSON，人可读 | 通常 protobuf 二进制，更紧凑 |
| 典型场景 | 面向浏览器/第三方的开放 API | 服务间内部调用 |

REST 你已经在 [《REST 接口设计》](../http/rest.md) 里系统学过了，
本课重点讲 RPC 这一侧——尤其是 gRPC 靠 IDL 强制约定契约、
以及“一元调用”之外的流式调用能力，这两点是它和普通 HTTP JSON
接口最大的差异。

对应 Go 侧调用同一个方法：

```go
resp, err := client.GetUser(ctx, &pb.GetUserRequest{Id: 123})
```

`client.GetUser` 和 `client.get_user` 长得就像调一个普通函数，
底层其实发生了一整套“序列化参数 → 通过 HTTP/2 发出 → 对方反序列化
→ 调用真正的处理函数 → 序列化返回值 → 传回来反序列化”，
只是这些步骤被生成的代码全部封装掉了，你完全感觉不到网络的存在——
这正是“RPC”这个名字的含义：**Remote Procedure Call，让远程调用
表现得像本地过程调用**。

----

# IDL 先定契约再生代码

> gRPC 要求你先用 protobuf 写一份接口定义（IDL，接口描述语言），
> 再用工具从这份定义生成两端的代码——**先有契约，后有实现**。

一份最小的 `.proto` 文件：

```protobuf
syntax = "proto3";
package demo;

service UserService {
  rpc GetUser (GetUserRequest) returns (GetUserResponse);
}

message GetUserRequest {
  int64 id = 1;
}

message GetUserResponse {
  int64 id = 1;
  string name = 2;
}
```

这份文件是 Rust 端和 Go 端**共享的唯一真相来源**：Rust 用
`tonic-build` 生成客户端和服务端的强类型代码，Go 用 `protoc-gen-go`
+ `protoc-gen-go-grpc` 生成对应代码，两端字段名、类型、方法签名
永远保持一致，不会出现“Go 那边加了个字段，Rust 那边没同步”的
低级错误——编译期就会报错，而不是等到线上跑出奇怪的反序列化 bug。

对比 REST：普通 JSON 接口通常靠一份文档（或者干脆没有文档）
双方手写结构体，字段名拼错、类型不一致这类问题只能在运行时暴露；
[《serde_json》](../http/serde-json.md) 里也提到过，反序列化时
字段不匹配往往是静默丢字段而不是报错，排查成本更高。**IDL 强制
两端从同一份定义生成代码，是 gRPC 相比手写 JSON 接口最大的
工程收益**。

生成代码后，Rust 端服务端实现长这样：

```rust
#[derive(Default)]
struct MyUserService;

#[tonic::async_trait]
impl UserService for MyUserService {
    async fn get_user(
        &self,
        request: tonic::Request<GetUserRequest>,
    ) -> Result<tonic::Response<GetUserResponse>, tonic::Status> {
        let id = request.into_inner().id;
        Ok(tonic::Response::new(GetUserResponse {
            id,
            name: "示例用户".into(),
        }))
    }
}
```

对应 Go：

```go
type myUserService struct {
    pb.UnimplementedUserServiceServer
}

func (s *myUserService) GetUser(ctx context.Context, req *pb.GetUserRequest) (*pb.GetUserResponse, error) {
    return &pb.GetUserResponse{Id: req.Id, Name: "示例用户"}, nil
}
```

两段代码的方法签名都是从同一份 `.proto` 生成的接口自动推出来的，
你只需要填方法体，参数类型、返回类型都不需要（也不应该）手写。

----

# 一元调用与流式调用

> 普通函数调用是“传参数、等返回值”，这叫一元调用（unary）；
> gRPC 额外支持三种流式调用，把“一次调用”变成“持续收发多条消息”。

四种调用模式：

| 模式 | 直觉 | 类比 |
| --- | --- | --- |
| 一元（unary） | 一个请求，一个响应 | 普通函数调用 |
| 服务端流式 | 一个请求，服务端持续推多个响应 | 订阅一个会持续更新的结果 |
| 客户端流式 | 客户端持续发多个请求，服务端最后给一个响应 | 边上传边处理，最后汇总 |
| 双向流式 | 两边都能随时发，互不等待 | 类似 [《WebSocket》](websocket.md) 的全双工通道，但消息有 protobuf 结构和方法边界 |

服务端流式在 `.proto` 里用 `stream` 关键字标记返回值：

```protobuf
service PriceService {
  rpc WatchPrice (WatchPriceRequest) returns (stream PriceUpdate);
}
```

Rust 端返回一个流：

```rust
async fn watch_price(
    &self,
    _request: tonic::Request<WatchPriceRequest>,
) -> Result<tonic::Response<Self::WatchPriceStream>, tonic::Status> {
    let (tx, rx) = tokio::sync::mpsc::channel(4);
    tokio::spawn(async move {
        loop {
            let update = PriceUpdate { price: fetch_latest_price() };
            if tx.send(Ok(update)).await.is_err() {
                break; // 客户端已经断开
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
    Ok(tonic::Response::new(ReceiverStream::new(rx)))
}
```

对应 Go：

```go
func (s *priceService) WatchPrice(req *pb.WatchPriceRequest, stream pb.PriceService_WatchPriceServer) error {
    for {
        update := &pb.PriceUpdate{Price: fetchLatestPrice()}
        if err := stream.Send(update); err != nil {
            return err // 客户端已经断开
        }
        time.Sleep(time.Second)
    }
}
```

双向流式和 [《WebSocket》](websocket.md) 的“帧”很像——都是一条
连接上双方随时互相发消息，区别在于 gRPC 每条消息仍然是强类型的
protobuf message，而不是裸的文本/二进制帧，方法边界（调用的是
哪个 RPC 方法）也是协议本身规定好的，不需要你自己在消息里塞
“类型字段”去区分。

----

# 错误码不是 HTTP 状态码

> gRPC 有自己的一套状态码，虽然底层跑在 HTTP/2 上，但业务错误
> 不是直接映射 HTTP 状态码，而是用一套专门为 RPC 设计的错误码。

常见的几个 gRPC 状态码：

| 状态码 | 含义 | 类比 REST |
| --- | --- | --- |
| `OK` | 成功 | `200` |
| `INVALID_ARGUMENT` | 请求参数不合法 | `400` |
| `NOT_FOUND` | 找不到目标资源 | `404` |
| `DEADLINE_EXCEEDED` | 调用超过设定的超时时间 | 客户端自己超时，REST 里通常没有专门状态码 |
| `UNAVAILABLE` | 服务暂时不可用，通常可以安全重试 | `503` |
| `INTERNAL` | 服务端内部错误 | `500` |

Rust 端返回错误：

```rust
if user_not_found {
    return Err(tonic::Status::not_found("用户不存在"));
}
```

对应 Go：

```go
if userNotFound {
    return nil, status.Errorf(codes.NotFound, "用户不存在")
}
```

`DEADLINE_EXCEEDED` 和 `UNAVAILABLE` 这两个码值得多说一句：
它们直接关系到“该不该重试”——`DEADLINE_EXCEEDED` 通常意味着
这次调用已经超过约定时间，重试前要想清楚上一次调用是否已经
产生了副作用（比如已经扣了款）；`UNAVAILABLE` 一般代表连接层面
的暂时性问题（服务重启、网络抖动），大多数场景下可以安全重试。
这部分“遇到哪种错误该不该重试、退避多久”的通用原则，
留给专门讲这个主题的《超时与重试》（`timeouts-retries.md`）。

----

# Rust tonic 与 Go grpc 库对照

一张速查表，方便你在两个生态之间切换时对上号：

| 事情 | Rust（`tonic`） | Go（`google.golang.org/grpc`） |
| --- | --- | --- |
| 生成代码 | `tonic-build`（build.rs 里调用） | `protoc-gen-go` + `protoc-gen-go-grpc` |
| 启动服务端 | `Server::builder().add_service(...).serve(addr).await` | `grpc.NewServer()` + `RegisterUserServiceServer` + `lis.Serve` |
| 创建客户端 | `UserServiceClient::connect(addr).await?` | `grpc.Dial(addr)` + `pb.NewUserServiceClient(conn)` |
| 设置超时 | `tonic::Request::new(req)` 配合 `tokio::time::timeout` 包裹调用 | `context.WithTimeout(ctx, d)` |
| 拦截器/中间件 | `tower::Layer`（gRPC 生态复用 HTTP 的 tower 中间件体系） | `grpc.UnaryServerInterceptor` |

最小的客户端调用对照：

```rust
let mut client = UserServiceClient::connect("http://127.0.0.1:50051").await?;
let response = client.get_user(GetUserRequest { id: 123 }).await?;
println!("{:?}", response.into_inner());
```

```go
conn, _ := grpc.Dial("127.0.0.1:50051", grpc.WithInsecure())
client := pb.NewUserServiceClient(conn)
resp, _ := client.GetUser(context.Background(), &pb.GetUserRequest{Id: 123})
fmt.Println(resp)
```

两段代码的结构完全一致：先拿到一个连接/客户端，再直接“调用方法”，
参数和返回值都是从 `.proto` 生成的强类型结构体——你几乎看不到
任何和网络相关的代码，这正是 RPC 想要的效果。

----

# 对照消息服务怎么读

> 这一节同样是**阅读指引**，不是教你部署什么。内部服务之间的调用，
> 很多时候并不是走浏览器熟悉的那种文本 HTTP，也不一定是标准
> gRPC，而是一套自定义的二进制协议——理解了这一点，你读那类源码
> 时才知道该往哪看。

浏览器发出的 HTTP 请求，你打开抓包工具就能直接读出方法、路径、
header，是纯文本；但很多内部长连接服务之间（不面向浏览器，
只服务于内部其它进程）为了效率，常常自定义一套二进制协议，
大致结构类似：

```
[ 帧长度 | 命令号(cmd) | 序列化方式 | 业务 payload ]
```

阅读这类代码时，建议按下面的顺序去找，而不是一头扎进业务逻辑：

1. 先找**帧头结构**的定义——通常是一个固定字节数的结构体，
   里面至少有“这一帧总长度”和“命令号（cmd id）”两个字段，
   这一步对应本课学的“契约”（只是这里的契约不是 `.proto`，
   而是一份自定义的二进制格式说明或代码里的常量定义）；
2. 再找**序列化边界**是怎么划分的——这条长连接上会连续收发
   很多帧，怎么知道一帧从哪到哪结束，通常就是靠上面提到的
   “帧长度”字段，思路和 [《TCP》](tcp.md) 讲的长度前缀完全一样；
3. 最后找**命令号到业务 handler 的分发表**——大多是一个
   `cmd id -> 处理函数` 的映射（有的项目叫 dispatch、
   有的直接是一个大 `switch`/`match`），本课举例常用占位名
   **dispatch**、**broker** 来代表“负责按某种标识做路由/转发”
   的这类角色，与 [《MQTT》](mqtt.md) “对照消息服务怎么读”一节
   提到的 dispatch/broker 是同一类角色，只是这里路由的依据是
   命令号，MQTT 里路由的依据是 topic。

对应到本课的概念，可以这样类比（同样是脱敏后的通用类比，
不代表任何具体产品的真实实现）：

| 本课概念 | 内部二进制协议里的对照 |
| --- | --- |
| IDL / `.proto` 契约 | 自定义帧头格式 + 命令号常量表，只是通常没有工具自动生成代码，需要手动维护两端一致 |
| 一元调用 / 流式调用 | 同一条长连接上可能既有“一问一答”式的命令，也有服务端持续推送的命令，靠命令号区分，而不是像 gRPC 那样在 `.proto` 里声明 `stream` |
| 错误码 | 内部协议通常有自己的一套错误码字段，不一定是 gRPC 的 `Status`，含义要看具体项目的约定 |

如果你手头有类似的内部服务代码仓库，建议自己在本地打开相关目录
（比如一个消息服务仓库里常见会拆出 `login`、`linker`、
`dispatch` 这类子目录，命名和 [《MQTT》](mqtt.md) 一课的占位名
呼应）逐个读一读帧头定义和分发表，比单看文档更容易建立直觉。
再强调一次：这里给的都是**占位名和通用结构**，不涉及任何真实
地址、端口、生产配置，具体到某个仓库的真实协议字段、命令号取值，
都以你本地代码为准。

----

# 一条 HTTP2 连接扛住所有调用

> gRPC 客户端通常只维护**一条**到某个服务端的 HTTP/2 连接，
> 所有并发的 RPC 调用都复用这一条连接——这是它和传统一个请求
> 一条连接模型最大的运行时差异。

回顾 [《HTTP2 卡在队头阻塞》](quic-http3.md) 讲的多路复用：
HTTP/2 允许一条连接上并发跑多个“流”，gRPC 直接借用了这个能力，
**每一次 RPC 调用对应一个 HTTP/2 流，多个调用可以同时在同一条
连接上并发进行，不需要为每个调用单独建一条新连接**。这带来
两个直接的好处，也带来一个容易被忽略的风险点：

- 好处一：省去了 [《TCP》](tcp.md) 讲过的重复三次握手和
  [《TLS 是 HTTP 的外壳》](tls.md) 的 TLS 握手开销，只在客户端
  第一次连接时付一次；
- 好处二：`tonic`/gRPC 客户端库内部已经处理好了“一条连接支撑
  多个并发调用”的细节，业务代码不需要自己管理连接池；
- 风险点：**如果这条唯一的连接出问题（网络抖动、对端重启），
  所有依赖它的并发调用会同时受影响**——这也是 HTTP/2 队头阻塞
  问题在 gRPC 场景下的具体体现，回顾
  [《HTTP2 卡在队头阻塞》](quic-http3.md) 的讲解，同一条连接上
  某个流的问题理论上不该拖累别的流，但连接本身整体断开时，
  所有流都会一起失效。

生产环境常见的应对方式是给客户端配置连接级的 keepalive
（区别于 [《TCP》](tcp.md) 讲的传输层 keepalive，这里是 gRPC/
HTTP2 协议层面的 ping），以及在连接异常时自动重连：

```rust
let endpoint = tonic::transport::Endpoint::from_static("http://127.0.0.1:50051")
    .keep_alive_while_idle(true)
    .http2_keep_alive_interval(std::time::Duration::from_secs(30));
let client = UserServiceClient::connect(endpoint).await?;
```

对应 Go：

```go
conn, err := grpc.Dial("127.0.0.1:50051",
    grpc.WithInsecure(),
    grpc.WithKeepaliveParams(keepalive.ClientParameters{
        Time:    30 * time.Second,
        Timeout: 10 * time.Second,
    }),
)
```

----

# Deadline 要跟着调用链传递

> gRPC 的超时机制叫 **deadline**（截止时间点，而不是“再等多久”
> 这种相对时长），这个设计选择解决了微服务调用链里一个真实的
> 传递问题。

回顾 [《超时和重试不是小事》](timeouts-retries.md) “Go context
与 Rust tokio::time::timeout 对照”一节强调的原则：**超时预算
应该从最外层往内层逐级分配**。gRPC 用绝对时间点（deadline）
而不是相对时长（timeout）来表达这个概念，好处是**这个截止
时间点可以原样传递给下游调用**，不需要每一层自己重新计算
“还剩多少时间”：

```
用户请求进来，设定 deadline = 现在 + 800ms
  │
  ▼
服务 A 调用服务 B：直接把同一个 deadline 传下去
  │（B 收到时，可能已经过去了 200ms，B 自己算出"还剩 600ms"）
  ▼
服务 B 调用服务 C：把同一个 deadline 继续传下去
```

Go 的 `context.Context` 天然支持这个模型——`context.WithDeadline`
设置的是一个绝对时间点，透过 `ctx` 一路传递给下游调用：

```go
deadline := time.Now().Add(800 * time.Millisecond)
ctx, cancel := context.WithDeadline(context.Background(), deadline)
defer cancel()
resp, err := client.GetUser(ctx, req) // deadline 会随 ctx 自动传递
```

Rust 的 `tonic` 也支持类似机制，通过在请求的元数据里显式携带
截止时间（`grpc-timeout` 是 gRPC 协议标准定义的 header）：

```rust
let mut request = tonic::Request::new(GetUserRequest { id: 123 });
request.set_timeout(std::time::Duration::from_millis(600)); // 剩余预算
let response = client.get_user(request).await?;
```

**没有做这层传递的常见后果**：外层服务早就超时放弃了，内层
服务却完全不知道，继续按自己独立设置的超时傻等或处理，白白
浪费资源——这正是
[《超时和重试不是小事》](timeouts-retries.md) 反复强调的
“预算从外层往内层逐级分配”原则在 RPC 调用链上的具体落地。

----

# 常见误区

> 从 REST/普通 HTTP 接口转到 gRPC，容易带错的几个假设。

- **误区一：gRPC 一定比 REST/JSON 快，所有场景都该换。**
  gRPC 在“服务间高频调用、字段结构复杂、需要强类型契约”的场景
  收益明显，但对浏览器直接调用、需要人工调试查看报文的场景
  （protobuf 二进制不像 JSON 那样直接可读），JSON REST 接口
  仍然更合适，选型要看场景而不是盲目追新；
- **误区二：`.proto` 改了字段之后，两端只要重新生成一次代码
  就一定兼容。** protobuf 确实支持一定程度的向前/向后兼容
  （比如新增字段、不删除已用的字段编号），但删除或修改已有
  字段编号的语义、把字段类型改成不兼容的类型，仍然会破坏
  兼容性——生成代码通过编译不代表运行时行为一定兼容；
- **误区三：`UNAVAILABLE` 一定可以放心重试。** 大多数场景下
  安全，但如果这次调用本身不是幂等操作（比如触发了下单），
  即使错误码提示“大概率是连接层面的暂时性问题”，仍然要按
  [《超时和重试不是小事》](timeouts-retries.md) 的幂等判断
  原则来决定要不要重试，错误码只是线索，不是最终判断依据；
- **误区四：gRPC 的流式调用和 WebSocket 是同一个东西，随便换。**
  两者都能做到双向随时收发，但 gRPC 的每条消息都严格遵循
  `.proto` 定义的强类型结构，方法边界由协议规定；WebSocket
  的帧本身没有这层结构约束，格式完全由业务自己在应用层约定，
  两者适用场景有重叠但不是可以随意互换的同一种技术。

----

# 排错对照

> gRPC 调用失败时，先看 gRPC 状态码，再结合下面的表格定位问题。

| 现象 | 大概率原因 | 排查方向 |
| --- | --- | --- |
| 客户端报 `DEADLINE_EXCEEDED` | 调用耗时超过了设置的 deadline | 检查 deadline 设置是否合理，或者下游服务是否变慢了 |
| 客户端报 `UNAVAILABLE`，偶发出现 | 连接暂时性问题（对端重启、网络抖动） | 检查连接的 keepalive 配置，确认对端服务健康状态 |
| 服务端和客户端各自的 `.proto` 版本不一致导致解析异常 | 两端代码生成的时机不同步，其中一端用了旧版本 `.proto` | 确认双方 `.proto` 文件版本一致，重新生成代码 |
| 高并发下所有调用同时失败 | 唯一的底层 HTTP/2 连接断开，所有并发的流一起受影响 | 检查客户端的连接重连和 keepalive 配置，回顾“一条 HTTP2 连接扛住所有调用”一节 |
| 流式调用里，客户端断开后服务端没有感知到 | 服务端没有正确处理 `tx.send`/`stream.Send` 的错误返回值 | 检查服务端流式发送逻辑是否在发送失败时正确退出循环 |

----

# 动手实验

1. 写一份最小的 `.proto` 文件（参考本课示例），分别用
   `tonic-build` 和 `protoc` 生成 Rust、Go 两端代码，对比生成
   出来的方法签名是否一一对应；
2. 实现一个一元调用的最小 echo 服务，用抓包工具观察实际网络层
   走的是 HTTP/2 帧，而不是纯文本 HTTP/1.1 报文；
3. 把上面的服务改成服务端流式，客户端调用一次，观察能收到多条
   响应，且客户端主动断开后服务端 `stream.Send`/`tx.send` 会报错
   （感知到对方已经不在了）；
4. 故意让服务端返回 `UNAVAILABLE` 和 `INVALID_ARGUMENT` 两种错误，
   在客户端分别处理，体会“这个错误该不该自动重试”的判断逻辑；
5. 如果你手头有内部二进制协议的服务代码，尝试照着本课“先找帧头、
   再找序列化边界、最后找分发表”的顺序读一遍，看能不能在
   10 分钟内定位到某个具体命令号对应的处理函数。

----

# 三句话带走

1. RPC 让远程调用长得像本地函数调用；gRPC 用 `.proto` 这份 IDL
   强制约定两端契约，编译期就能发现字段不一致，比手写 JSON
   接口更不容易踩坑。
2. gRPC 除了一元调用，还支持服务端流式、客户端流式、双向流式，
   双向流式和 WebSocket 的全双工很像，区别是消息始终是强类型
   protobuf message，方法边界由协议本身规定。
3. gRPC 有自己的一套状态码（如 `DEADLINE_EXCEEDED`、
   `UNAVAILABLE`），不是直接照搬 HTTP 状态码；内部服务间常用
   自定义二进制协议，阅读时先找帧头和命令号，再找分发表定位
   业务逻辑。

----

# 附：本课生词表

- **RPC（远程过程调用）** —— 让调用远程服务的代码写法接近调用
  本地函数的一类技术。
- **IDL（接口描述语言）** —— 独立于具体语言、用来定义服务方法和
  消息结构的描述文件，如 `.proto`。
- **一元调用（unary）** —— 一次请求对应一次响应的调用方式。
- **流式调用（streaming）** —— 一次调用中可以持续收发多条消息，
  分服务端流式、客户端流式、双向流式。
- **gRPC 状态码** —— gRPC 专用的错误码体系，如
  `DEADLINE_EXCEEDED`、`UNAVAILABLE`，与 HTTP 状态码不是一一对应。
- **命令号（cmd id）** —— 自定义二进制协议里标识“这一帧是什么
  操作”的字段，作用类似 gRPC 里的方法名。
- **帧头（frame header）** —— 自定义协议里描述一帧数据长度、类型
  等元信息的固定结构，用于划分消息边界。
- **分发表（dispatch table）** —— 把命令号/主题等标识映射到具体
  处理函数的查找表。
- **deadline** —— gRPC 用绝对时间点表达的超时机制，可以原样
  传递给调用链下游，不需要每一层重新计算剩余时间。
- **`grpc-timeout`** —— gRPC 协议里携带 deadline/剩余超时预算
  的请求元数据字段。
