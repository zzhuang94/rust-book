# 网络集成测试

> 代码：`code/engineering-network-testing/`　运行：  
> `cargo test -p engineering-network-testing`

[《测试》](testing.md) 已经讲过纯函数、Tokio 时间快进和 Axum oneshot。
oneshot 很快，但它不会真正经过端口、TCP、HTTP 编解码和客户端超时。

本课再向外走一层：测试进程在 127.0.0.1:0 启动临时假服务，
生产客户端通过真实网络请求它，并主动制造非 2xx、坏 JSON、慢响应、断连和 UDP 无回包。

所有地址和数据都由测试临时生成，不连接公网，也不依赖共享测试环境。

----

# 先选测试边界

不是所有测试都应该开端口。边界越真实，覆盖越多，成本也越高：

| 测试方式 | 穿过的边界 | 适合验证 |
| --- | --- | --- |
| 纯函数测试 | 无网络 | 解析、计算、筛选 |
| Router oneshot | Axum 路由和 handler | 状态码、提取器、响应体 |
| 本地真实 HTTP | TCP + HTTP + 客户端 | 超时、连接、协议和 JSON |
| 外部环境测试 | 真实依赖和部署网络 | 配置、证书、权限、兼容性 |

本课选择第三层。它比 oneshot 慢一点，却能发现：

- URL 或端口拼错；
- 客户端没有设置超时；
- 500 被误当成成功；
- content-type 和响应体不匹配；
- 对端连接中途断开；
- 服务启动存在端口竞争。

它仍然不应该替代单元测试。正确结构通常是大量便宜测试，加少量关键网络集成测试。

----

# 生产代码不识假服务

被测函数只接收 Client 和 URL：

~~~rust
pub async fn fetch_snapshot(
    client: &reqwest::Client,
    url: &str,
) -> anyhow::Result<Snapshot> {
    client
        .get(url)
        .send()
        .await
        .context("请求上游失败")?
        .error_for_status()
        .context("上游返回失败状态")?
        .json::<Snapshot>()
        .await
        .context("上游响应不是合法快照")
}
~~~

生产环境传真实 URL，测试传临时假服务 URL。生产代码不需要出现：

- 测试模式开关；
- 假响应分支；
- 固定测试端口；
- 只在测试时改变业务行为的条件判断。

这叫从依赖边界注入配置。替换的是服务地址，不是复制一份生产实现。

对照 Go：

~~~go
func fetchSnapshot(
    client *http.Client,
    url string,
) (Snapshot, error)
~~~

两边的可测试性原则一样：不要把 URL 和客户端构造硬编码在函数深处。

----

# 端口零避免冲突

测试服务这样绑定：

~~~rust
let listener = TcpListener::bind("127.0.0.1:0").await?;
let addr = listener.local_addr()?;
~~~

端口 0 表示让操作系统挑一个当前空闲端口。它解决三个问题：

1. 开发机不必提前腾出固定端口；
2. 多个测试可以并行运行；
3. CI 上不同任务不会争抢同一个端口。

绑定后必须通过 local_addr() 取回真实端口，再拼测试 URL。

不要先随机生成一个端口号，再检查它是否空闲。从“检查空闲”到“真正绑定”之间，
另一个进程可能抢走它，这叫 TOCTOU 竞争。让操作系统在 bind 时原子分配才可靠。

----

# 先 bind 再 spawn

一个经典脆弱写法：

~~~rust
tokio::spawn(start_server());
tokio::time::sleep(Duration::from_millis(100)).await;
// 猜服务应该启动好了
~~~

这会产生偶发失败：电脑快时浪费 100ms，CI 忙时 100ms 又不够。

本课先在当前任务完成 bind，再把已经绑定的 listener 交给后台服务：

~~~rust
let listener = TcpListener::bind("127.0.0.1:0").await?;
let addr = listener.local_addr()?;
let task = tokio::spawn(async move {
    axum::serve(listener, app).await.unwrap();
});
~~~

bind 成功就说明端口已经被本测试占住。后续连接即使比 Axum 接收循环更早到达，
也会先进入操作系统监听队列，不需要猜启动时间。

> 🔩 底层视角：就绪同步应依赖“资源已经取得”这个事实，而不是依赖墙上时钟过了多久。
> sleep 只能表达等待，不能证明条件成立。

----

# 夹具负责回收

测试创建后台服务后，必须保证测试成功或 panic 时都能清理。
示例用一个小结构持有任务：

~~~rust
struct TestServer {
    url: String,
    task: JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
~~~

这个结构叫测试夹具（fixture）：

- url 告诉测试去哪里请求；
- task 保住后台任务的管理权；
- 离开作用域自动 abort，不要求每个测试记得手动清理。

对照 Go 的 httptest.Server.Close() 或 t.Cleanup(server.Close)。
Rust 用 Drop 把清理动作绑定到所有权生命周期。

真实项目还可以让夹具保存临时目录、数据库 schema 或环境变量恢复器。
原则是：谁创建资源，谁负责在所有退出路径回收。

----

# 成功路径先钉住

先写一个正常响应，证明测试基础设施本身可用：

~~~rust
let app = Router::new().route(
    "/snapshot",
    get(|| async {
        Json(json!({"version": 7, "nodes": ["node-a"]}))
    }),
);
let server = serve(app).await;

let snapshot = fetch_snapshot(&client, &server.url).await.unwrap();
assert_eq!(snapshot.version, 7);
~~~

为什么故障测试前要有成功测试？如果所有场景都失败，你无法判断：

- 被测代码真的正确识别故障；
- 还是假服务根本没启动；
- URL 拼错；
- 测试客户端配置错误。

成功路径是整套测试线路的“自检”。

----

# 非二百单独测

HTTP 请求成功到达，不代表业务成功。假服务返回 503：

~~~rust
get(|| async {
    (StatusCode::SERVICE_UNAVAILABLE, "稍后再试")
})
~~~

客户端必须调用 error_for_status()，否则 503 仍是正常的 reqwest::Response，
后续代码可能继续解析正文。

断言不要依赖 reqwest 完整英文报错：

~~~rust
let error = fetch_snapshot(&client, &server.url)
    .await
    .unwrap_err();
assert!(format!("{error:#}").contains("失败状态"));
~~~

这里断言自己添加的稳定业务上下文，而不是第三方库某个版本的全部错误字符串。
如果代码提供结构化错误枚举，更推荐直接用 matches! 断言错误变体。

----

# 坏 JSON 测协议

状态码 200，但正文故意残缺：

~~~rust
get(|| async {
    ([("content-type", "application/json")], "{broken")
})
~~~

这个场景验证两件事：

1. 客户端确实尝试按约定类型反序列化；
2. 解析失败会带上“上游响应不是合法快照”的现场。

还可以继续增加：

- 缺少必填字段；
- 字段类型错误；
- 空数组是否允许；
- 数组超过业务上限；
- JSON 合法但业务版本倒退。

Serde 只保证数据形状能匹配 Rust 类型。业务合法性仍要在解析后检查。

----

# 慢响应测总超时

假服务先等待，再返回合法 JSON：

~~~rust
get(|| async {
    tokio::time::sleep(Duration::from_millis(500)).await;
    Json(json!({"version": 8, "nodes": []}))
})
~~~

客户端超时设为 50ms，预期请求失败。

时间类测试最容易 flaky，遵守四条：

- 慢服务等待时间明显大于客户端预算，最好至少 5 到 10 倍；
- 不断言精确耗时“必须是 50ms”；
- 只断言它在合理上限内失败；
- 不把系统极度繁忙误判为业务错误。

Tokio 的暂停时间对真实网络 IO 帮助有限。虚拟时钟能快进 sleep，
但 TCP 就绪仍受操作系统控制。网络超时测试使用真实时间，同时保持预算短而差距足够大。

----

# 断连测传输错误

Axum 总会尽力返回合法 HTTP。如果要模拟“TCP 已连接，但一个 HTTP 字节都没返回就断开”，
直接使用 TcpListener 更准确：

~~~rust
let (socket, _) = listener.accept().await?;
drop(socket);
~~~

客户端可能报告 EOF、连接重置或请求失败，具体底层文字受操作系统影响。
因此测试只断言：

- 结果是 Err；
- 外层错误属于“请求上游失败”；
- 没有把空响应解析成默认值。

不要断言 Windows 必须出现某句话、Linux 必须出现另一句话。
跨平台测试应该验证语义，不要绑定内核措辞。

----

# UDP 无回包

UDP 没有连接，也没有“服务器已断开”通知。对端不回包时，客户端唯一可靠的结束条件
通常是超时。

测试绑定一个真实 UDP 端口，但故意不读取也不回包：

~~~rust
let silent_peer = UdpSocket::bind("127.0.0.1:0").await?;
let peer_addr = silent_peer.local_addr()?;

client.send_to(b"ping", peer_addr).await?;
let result = timeout(
    Duration::from_millis(30),
    client.recv_from(&mut buf),
).await;

assert!(result.is_err());
~~~

保持 silent_peer 活到断言结束很重要。如果提前 drop，某些平台可能立即产生不同的
端口不可达行为，测试语义就从“无响应”变成了“目标不存在”。

还可以测试：

- 回包来源地址不符合预期；
- 回包被截断；
- 消息 ID 不匹配；
- 收到重复包；
- 先收到无效包，再收到有效包。

----

# 故障矩阵先列全

不要想到一个错误就随手加一个测试。先按层列矩阵：

| 层次 | 正常 | 故障 |
| --- | --- | --- |
| 地址 | 合法 URL | 非法 URL、DNS 失败 |
| TCP | 成功连接 | 拒绝、断连、连接超时 |
| HTTP | 2xx | 4xx、5xx、重定向 |
| 响应体 | 合法 JSON | 空体、坏 JSON、超大正文 |
| 业务协议 | 字段有效 | 版本倒退、重复节点、空数据 |
| 时间 | 按时返回 | 慢首字节、慢响应体、永久无回包 |

然后按风险选代表场景。不是每个组合都要测试，但关键故障必须有人负责。

例如“500 + 坏 JSON”通常只需验证状态码优先，因为客户端不应该在 500 后继续解析；
“200 + 坏 JSON”才真正覆盖解析错误。

----

# 重试测试看次数

如果被测代码有重试，假服务应记录调用次数：

~~~rust
let calls = Arc::new(AtomicUsize::new(0));
~~~

handler 可以按次数返回：

~~~text
第 1 次 → 503
第 2 次 → 503
第 3 次 → 200
~~~

测试应断言：

- 最终成功；
- 总调用次数正好是 3；
- 不可重试错误只调用 1 次；
- 达到总预算后停止；
- 取消信号到来后不再发新请求。

不要只断言“最终成功”。如果代码悄悄重试了 100 次，结果虽然成功，行为仍然危险。

带随机抖动的退避应把随机数源或等待策略注入测试，避免测试依赖真实随机时间。

----

# 并行测试要隔离

Rust 测试默认并行。网络测试应做到：

- 每个测试独立绑定端口 0；
- 不共享可变全局行为开关；
- 每个测试创建自己的假服务和 Client；
- 测试结束自动回收任务；
- 不依赖执行顺序。

一个常见坏设计是所有测试共用固定端口，再用全局变量决定下一次返回什么。
并行时，测试 A 可能吃到测试 B 的响应，最终变成“单独成功、整套偶发失败”。

如果确实要共享昂贵资源，应使用唯一命名空间隔离数据，而不是默认所有测试串行。

----

# 不连共享环境

日常集成测试不应依赖团队共享服务器：

- 数据会被别人修改；
- 网络抖动导致无关失败；
- 测试可能误删真实数据；
- CI 需要额外凭证；
- 失败后很难复现当时状态。

本地假服务的优势是快、确定、可制造故障。共享环境适合更上层的部署验收，
并且应使用专用账号、专用数据和明确清理策略。

测试日志和 fixture 中同样不能放真实令牌、内部域名、生产 IP 或用户数据。

----

# 常见错误还原

## sleep 等服务启动

时间过去不代表条件成立。先 bind listener，再 spawn 服务。

## 写死测试端口

并行和 CI 容易冲突。绑定 127.0.0.1:0 并读取实际端口。

## 只测成功路径

网络代码的大部分复杂度在失败路径。至少覆盖状态码、解析、超时和断连。

## 精确匹配系统错误

底层错误文字跨平台、跨版本会变化。断言稳定的业务错误类别和上下文。

## 测试后不关服务

孤儿任务会污染后续测试。用 Drop 夹具或明确的取消令牌回收。

## 用 mock 复制实现

如果 mock 直接返回 Snapshot，就跳过了 TCP、状态码和 JSON。
需要验证网络边界时，应让客户端请求真实本地 socket。

----

# 动手实验

1. 删除客户端的 error_for_status()，观察 503 场景转而出现什么错误；
2. 把测试端口改成固定值，同时启动两份测试，体会端口冲突；
3. 增加“200 + 缺少 version”场景，观察 Serde 错误链；
4. 实现前两次 503、第三次成功的假服务，并断言请求次数；
5. UDP 假服务先发错误 ID，再发正确 ID，验证客户端会忽略无关数据报。

----

# 三句话带走

1. oneshot 测应用内部，绑定 127.0.0.1:0 的假服务测试真实 TCP/HTTP 边界。
2. 先 bind 再 spawn，不用 sleep 猜就绪；夹具 Drop 自动回收后台服务。
3. 故障测试覆盖非 2xx、坏数据、慢响应、断连和无回包，并断言稳定语义而非系统措辞。

----

# 附：本课生词表

- **网络集成测试** —— 穿过真实本地 socket 和协议编解码的自动化测试。
- **测试边界** —— 一次测试实际覆盖到系统的哪一层。
- **假服务（fake server）** —— 由测试控制响应和故障行为的临时本地服务。
- **fixture** —— 为测试创建并回收依赖资源的夹具。
- **端口 0** —— 请求操作系统在 bind 时分配空闲临时端口。
- **TOCTOU** —— “检查时”和“使用时”之间状态被改变的竞争。
- **flaky test** —— 代码没变却偶发成功或失败的不稳定测试。
- **故障注入** —— 主动制造超时、断连、错误响应等条件，验证系统行为。
- **传输错误** —— 发生在 DNS、TCP、TLS 或字节收发层的错误。
- **协议错误** —— 状态码、格式或字段不符合双方约定。
- **测试矩阵** —— 按层次系统列出正常与故障组合的检查表。
- **孤儿任务** —— 创建者不再持有管理权、仍在后台运行的任务。
