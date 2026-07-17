# 排障从这几个工具开始

> 代码：`code/network-debug-tools/`　运行：`cargo run -p network-debug-tools`  

前面几课讲的是“正常情况下网络应该怎么工作”，这一课反过来，  
讲“东西坏了，该用什么工具去看”。这是网络系列的最后一课，  
把前面每一课提到过的排障动作（`curl -v`、`netstat`、  
[《TCP》](tcp.md) 讲的握手/RST/超时现象）串成一套完整的  
排障流程，附一张决策表，方便你线上出问题时照着走。  

本课不要求装 Wireshark 才能读——大部分场景 `curl` 和  
系统自带的连接查看工具就够用，Wireshark/tcpdump 放在  
“更深一层”的位置，需要时再动用。  

----

# curl -v 是第一件武器

> 遇到任何“接口不通”的问题，第一件事永远是用 `curl -v`  
> 自己发一次请求，而不是先怀疑代码逻辑。  

[《HTTP 是文本协议》](http-protocol.md) 已经用过 `curl -v`  
看协议原文，这里从排障角度重新过一遍它能告诉你什么：  

```bash
curl -v http://localhost:8080/api/health
```

`-v` 输出里，每一段代表排障链路上的一个阶段，从上到下依次是：  

```
*   Trying 127.0.0.1:8080...        ← ① 正在尝试建立 TCP 连接
* Connected to localhost (127.0.0.1) port 8080  ← ② TCP 连接建立成功
> GET /api/health HTTP/1.1          ← ③ 请求已经发出去
> Host: localhost:8080
>
< HTTP/1.1 200 OK                   ← ④ 收到了响应状态行
< Content-Length: 15
<
{"status":"ok"}                    ← ⑤ 收到了完整 body
```

排障时最有用的地方是：**卡在哪一步、报什么错，直接对应故障发生  
在哪一层**。下一节会把这四种典型故障和对应的报错逐一对上号。  

其他几个排障时常用的 curl 参数：  

```bash
curl -v --connect-timeout 3 http://localhost:8080/   # 只给连接阶段 3 秒
curl -v -k https://localhost:8443/                    # 跳过证书校验（仅调试用）
curl -v -o /dev/null -w "%{time_total}\n" http://localhost:8080/  # 只看总耗时
curl -v -I http://localhost:8080/                     # 只发 HEAD，只看响应头
```

----

# 看监听和连接：ss / netstat

> curl 告诉你“这次请求发生了什么”，`ss`/`netstat` 告诉你  
> “这台机器上现在有哪些端口在监听、哪些连接已经建立”。  

先确认目标服务到底有没有在监听你以为的端口——这是排障时  
经常被跳过、但特别值得先做的一步。  

Linux 上推荐用 `ss`（`netstat` 的现代替代品，速度更快）：  

```bash
ss -tlnp        # -t TCP，-l 只看监听中的，-n 不解析域名，-p 显示进程
```

典型输出：  

```
State   Local Address:Port   Peer Address:Port  Process
LISTEN  0.0.0.0:8080          0.0.0.0:*           pid=1234,name=("myapp")
```

看当前已建立的连接（比如想确认某个客户端有没有连上来）：  

```bash
ss -tn state established
```

Windows 上对应用 `netstat`（PowerShell 或 cmd 都能跑）：  

```powershell
netstat -ano | findstr LISTEN     # 查看监听中的端口
netstat -ano | findstr ESTABLISHED # 查看已建立的连接
```

`-ano` 里的 `-o` 会显示进程 ID（PID），配合任务管理器或  
PowerShell 的 `Get-Process -Id <PID>`，能反查这个端口具体是  
哪个进程占用的。Windows 也有原生的 PowerShell 写法：  

```powershell
Get-NetTCPConnection -State Listen        # 查看监听中的端口
Get-NetTCPConnection -State Established   # 查看已建立的连接
```

这一步能帮你回答几个最基础但很关键的问题：  

- 服务真的启动了吗？端口真的绑定成功了吗？  
- 绑定的是 `127.0.0.1`（只有本机能连）还是 `0.0.0.0`  
  （所有网卡都能连）——这个区别经常是“本机能访问、  
  别的机器访问不了”的真正原因；  
- [《TCP》](tcp.md) 讲过的 `TIME_WAIT`，也能在这里的输出里  
  直接看到数量——如果堆积了大量 `TIME_WAIT`，通常是短连接  
  压力大的正常现象，不一定是 bug。  

----

# 抓包看什么：SYN/FIN/RST/重传

> 前面两个工具解决大多数问题；如果还是看不出问题在哪，  
> 才需要抓包，直接看网线上到底跑了什么。  

Wireshark（图形界面）和 `tcpdump`（命令行，Linux/macOS 常见）  
抓到的都是同一份原始数据——网卡收发的每一个包。排障时不需要  
逐字节读懂所有内容，重点看这几个标志位：  

| 标志位 | 含义 | 排障意义 |
| --- | --- | --- |
| `SYN` | 请求建立连接（三次握手第一步） | 客户端发出去了 SYN，一直没有回应——网络不通或对方丢包 |
| `SYN, ACK` | 同意建立连接 | 服务端确实收到了、也回应了——说明网络层没问题 |
| `RST` | 强制中断连接 | 常见于“对方压根没有进程在监听这个端口”，回顾 [《TCP》](tcp.md) 三次握手一节 |
| `FIN` | 正常关闭一个方向 | 四次挥手的正常流程，不代表异常 |
| 重传（retransmission） | 同一个包被重新发了一次 | 说明上一次发出去的包被判定为丢了，网络质量有问题 |

命令行最小示例（Linux，抓本机 8080 端口相关的包）：  

```bash
sudo tcpdump -i any port 8080 -n
```

Windows 上没有原生 `tcpdump`，通常用 Wireshark 的图形界面直接  
抓包；也可以借助 WSL（Windows Subsystem for Linux）跑  
`tcpdump`，两种方式都可行。  

一个排障时非常实用的读法：**先看有没有 SYN 却没有 SYN-ACK**——  
如果有，说明包发出去了、但对方完全没回应，问题大概率在网络层  
或者对方机器整个不可达，而不是对方进程的业务逻辑；**如果 SYN  
之后很快收到 RST**，说明网络本身是通的，只是对方端口没人监听。  

> 🔬 底层视角：抓包工具能看到的是“经过这个网卡的包”，如果问题  
> 发生在两个都不是你能抓包的机器之间（比如客户端和服务端都在  
> 云上，你只能登录客户端），你能抓到的只是“客户端发出去的部分”，  
> 看不到“服务端到底收到了什么”——这也是为什么复杂问题经常需要  
> 两端同时抓包对照时间戳，才能确认包到底丢在哪一段。  

----

# 四种失败怎么区分

> 业务代码里“请求失败了”背后至少有四种完全不同的原因，  
> 排查方向也完全不同，先分清是哪一种，比急着改代码更重要。  

- **DNS 解析失败**：连域名对应的 IP 都没查到，通常发生在  
  “连接”这一步**之前**，[《寻址》](addressing.md) 讲过域名  
  要先解析成 IP 才能建立连接；  
- **连接被拒绝（connection refused）**：目标机器本身是可达的，  
  但没有进程在监听目标端口——[《TCP》](tcp.md) 讲过，这种情况  
  下 `connect` 通常很快就会报错，不会一直卡住；  
- **连接/读写超时（timeout）**：请求发出去了，但迟迟没有任何  
  回应——[《超时和重试不是小事》](timeouts-retries.md) 讲过，  
  这可能是网络不通（包被静默丢弃）、也可能是对方在处理但很慢；  
- **TLS 握手失败**：TCP 连接本身建立成功了，但 [《TLS 是 HTTP  
  的外壳》](tls.md) 讲的证书验证、加密参数协商这一步没通过。  

用命令输出的具体特征，来区分到底是哪一种：  

| 现象 | curl -v 报错关键字 | 大概率原因 |
| --- | --- | --- |
| 卡在 `Trying ...` 很久之后报错 | `Connection timed out` | 网络不通，包被静默丢弃；或对方防火墙拦截 |
| 几乎立刻报错，没有明显等待 | `Connection refused` | 目标机器可达，但没进程监听这个端口 |
| 报错提到域名解析 | `Could not resolve host` | DNS 解析失败，问题发生在连接之前 |
| TCP 连接建立提示成功，随后报错 | `SSL certificate problem` / `certificate verify failed` | TLS 握手阶段的证书验证失败 |
| TCP 连接成功，请求发出去后长时间无响应 | 卡住直到超时，或 `Empty reply from server` | 对方连上了但处理很慢，或处理完没有正常回应 |

这张表能帮你在看到一条报错时，快速把注意力定位到该往哪一层  
去查——DNS 问题去查域名解析配置，连接被拒绝去确认对方服务  
有没有启动，超时去查网络链路和对方负载，TLS 失败去查证书配置。  

----

# 排障决策表

> 把上面几节串成一张可以直接照着走的流程表，从“报告问题”  
> 到“定位到具体环节”。  

| 第几步 | 该做什么 | 用什么工具 | 观察什么 |
| --- | --- | --- | --- |
| 1 | 自己重现一次请求 | `curl -v` | 卡在哪一步、报什么错关键字 |
| 2 | 确认目标服务在监听 | `ss -tlnp` / `netstat -ano` | 端口有没有被正确的进程绑定 |
| 3 | 确认地址解析正常 | `curl -v`（看是否卡在解析域名） | 是否报 `Could not resolve host` |
| 4 | 确认网络层可达 | `ping`（仅测网络层，不代表应用层通） | 是否有回包、丢包率 |
| 5 | 确认端口能连上 | `curl -v` 的连接阶段 / `telnet 目标 端口` | 是 `refused` 还是卡住超时 |
| 6 | 如果是 HTTPS，单独排查证书 | `curl -v` 的握手信息 | 证书主题、有效期、签发者是否符合预期 |
| 7 | 仍无头绪，抓包看原始报文 | Wireshark / `tcpdump` | 有没有 SYN 无 SYN-ACK、是否频繁重传 |

建议按顺序从第 1 步走，**不要跳步**——很多时候看似“代码 bug”  
的问题，第 2 步就发现是服务压根没启动，或者绑的是  
`127.0.0.1` 而不是 `0.0.0.0`，根本用不上后面几步。  

----

# Windows 与 Linux 对照

> 前面各节的命令主要以 Linux 为例，这里补一张 Windows  
> 读者能直接用的对照表。  

| 目的 | Linux 命令 | Windows 对应 |
| --- | --- | --- |
| 看监听端口 | `ss -tlnp` | `netstat -ano \| findstr LISTEN` 或 `Get-NetTCPConnection -State Listen` |
| 看已建立连接 | `ss -tn state established` | `netstat -ano \| findstr ESTABLISHED` |
| 按端口找进程 | `lsof -i :8080` | `netstat -ano \| findstr :8080`（取 PID 后 `Get-Process -Id <PID>`） |
| 测试网络层可达 | `ping 目标` | `ping 目标`（用法基本一致） |
| 测试端口能否连上 | `nc -zv 目标 端口` 或 `telnet` | `Test-NetConnection -ComputerName 目标 -Port 端口` |
| 抓包 | `tcpdump` | Wireshark（图形界面）或 WSL 里跑 `tcpdump` |
| 实时看连接数变化 | `watch ss -tn` | Resource Monitor（资源监视器）的“网络”标签页 |

Windows 自带的 **Resource Monitor（资源监视器）** 值得单独提一下：  
打开方式是任务管理器 →“性能”标签页 → 底部“打开资源监视器”，  
里面的“网络”标签页能实时看到每个进程的连接数、发送/接收字节数，  
适合排查“这个进程到底有没有在正常收发数据”这类问题，图形化、  
不需要记命令，对刚接触网络排障的读者会更友好；`Test-NetConnection`  
则是 PowerShell 里最接近 `telnet 目标 端口` 的现代替代品，  
能直接给出“端口通不通”的明确结论。  

----

# 动手实验

1. 用 `curl -v` 分别请求一个正常运行的服务、一个确定没启动的  
   端口、一个不存在的域名，对照“四种失败怎么区分”那张表，  
   确认三次报错的关键字和表格描述的一致；  
2. 用 `ss -tlnp`（或 Windows 的 `netstat -ano`）确认某个正在  
   运行的示例服务确实绑定在预期的端口和地址上；  
3. 把一个测试服务绑定在 `127.0.0.1` 而不是 `0.0.0.0`，尝试从  
   同一台机器的另一个终端用 `curl` 访问，确认能连上；再尝试  
   （如果有第二台机器或容器）从外部访问，确认连不上，理解  
   两种绑定地址的区别；  
4. 如果装了 Wireshark，对着一次正常的请求和一次连接被拒绝的  
   请求分别抓包，对比两次抓包里 `SYN`/`RST` 出现的位置差异；  
5. 按“排障决策表”从第 1 步开始，完整走一遍某个你故意制造出来  
   的故障（比如先停掉服务再发请求），确认每一步观察到的现象  
   和表格描述吻合。  

----

# 三句话带走

1. 排障永远从 `curl -v` 自己重现一次请求开始，卡在哪一步、  
   报什么错，直接对应故障发生在哪一层，不要凭感觉先改代码。  
2. DNS 失败、连接拒绝、超时、TLS 握手失败是四种完全不同的  
   故障，报错关键字和现象各不相同，分清类型再决定往哪查。  
3. `ss`/`netstat`（或 Windows 的 `netstat`/`Get-NetTCPConnection`）  
   解决“端口有没有监听、连接有没有建立”，Wireshark/`tcpdump`  
   是更深一层的最后手段，看 `SYN`/`FIN`/`RST`/重传这几个标志位。  

----

# 附：本课生词表

- **`curl -v`** —— 打印请求和响应的详细过程，排障的第一件工具。  
- **`ss` / `netstat`** —— 查看本机 TCP 监听端口和已建立连接的  
  命令行工具，`ss` 是 Linux 上更现代的替代品。  
- **`Get-NetTCPConnection`** —— Windows PowerShell 里查看 TCP  
  连接状态的原生命令。  
- **`Test-NetConnection`** —— Windows PowerShell 里测试端口  
  是否可连通的命令，类似 `telnet`。  
- **Resource Monitor（资源监视器）** —— Windows 自带的图形化  
  工具，可实时查看各进程的网络连接和流量。  
- **`SYN` / `SYN-ACK` / `RST` / `FIN`** —— TCP 报文里的标志位，  
  分别对应“请求连接”“同意连接”“强制中断”“正常关闭一个方向”。  
- **重传（retransmission）** —— 同一个包被判定为丢失后重新  
  发送一次，是网络质量不佳的直接证据。  
- **连接被拒绝（connection refused）** —— 目标机器可达但没有  
  进程监听目标端口，通常很快报错，不会长时间卡住。  
- **DNS 解析失败** —— 域名查不到对应的 IP 地址，发生在建立  
  连接之前的更早阶段。  
- **TLS 握手失败** —— TCP 连接已建立，但证书验证或加密参数  
  协商没有通过。  
