# 改造大纲提案 —— async-lab → 面向 Go/Gin 开发者的 Rust 手册

> 本文件是 **改造路线图**。批次 0（骨架）与批次 1（结构迁移）已完成；余下批次 2–5 为「逐章填正文」，每批交付后你验收再继续。
>
> 定调（已确认）：**以服务端 / async 为主线，适度扩边**； **原地改造 async-lab**； **先骨架 + 大纲，确认后逐章填**。
>
> 结构决策（已确认）：主题目录 **收进 `docs/` 下**；侧栏 **不带编号**（照 cpp-book）；新增章节 = **A 组 3 篇 + B4 模式匹配 + B5 泛型 + B6 Cargo 生态**，共 6 篇。现有 25 篇全部保留扩写，全书 31 章。

----

# 一、已完成（批次 0 骨架 + 批次 1 结构迁移）

- **代码迁移**：14 个 crate + `labkit` + `Cargo.toml/Cargo.lock/target` 全部迁入 `code/`。工作空间成员路径相对 `code/Cargo.toml`，未改一行内容；`labkit` 的 `path` 依赖照旧解析。将来在 Linux 上 `cd code && cargo build` 即可（本机不跑构建，遵循既定校验流程）。
- **docsify 站点骨架**（照搬 cpp-book 形态）：
  - `index.html`：标题「Rust 手册」，Prism 语言组件换成 `rust / go / toml / bash / sql / json / yaml / powershell / docker`，保留 mermaid、页内 TOC、搜索、复制按钮、侧栏折叠。
  - `assets/custom.css`、`assets/toolbar.js`：原样复用（主题/字号切换、三套配色），另加了一段 Rust/Go 语法高亮。
  - `.nojekyll`、`favicon.ico`：GitHub Pages 需要的空标记 + 站点图标。
- **文档结构迁移（批次 1）**：现有 25 篇 `docs/*.md` 已 `mv` 进 `docs/{start,lang,concurrency,async,http,engineering,appendix}/` 并改用最终文件名；6 篇新增章节已建占位（含标题 + 本章规划要点）；跨章链接与源码链接已按新路径全部修正校验通过。
- **最终 `_sidebar.md`**：七组、31 章、无编号，全部指向最终路径。
- **`README.md`**：重写为「本书导读」（谁适合读、七组结构表、阅读路线、运行说明、Go→Rust 心智表、本地预览与部署）。

> **现在即可本地预览**：项目根执行 `docsify serve .`（或任意静态服务器），也可直接推 GitHub Pages。新增 6 章为占位页，正文将在批次 2 起逐章补齐。

----

# 二、目标目录结构

根目录 = docsify 站点；`docs/` 下按主题分子目录；`code/` = 可运行的 Rust 工作空间。

```
async-lab/
├── index.html          # docsify 配置
├── README.md           # 首页 / 简介（本书导读）
├── _sidebar.md         # 分组侧边栏（最终版）
├── .nojekyll
├── favicon.ico
├── assets/             # custom.css, toolbar.js
├── docs/
│   ├── start/          # 开始
│   ├── lang/           # 语言地基
│   ├── concurrency/    # 并发基础
│   ├── async/          # 异步主线
│   ├── http/            # HTTP 服务
│   ├── engineering/    # 工程实践
│   └── appendix/       # 附录
└── code/               # Rust 工作空间（14 crate + labkit）
    ├── Cargo.toml
    ├── labkit/
    └── async-basics/ … http-sqlx/
```

----

# 三、完整目录大纲（逐章映射）

标注：`← NN` 表示由现有 `docs/NN` 迁入并 **扩写**（只增不减）；`【新增】` 为本次新写。

## 开始 `start/`

| 文件 | 章节 | 来源 |
| --- | --- | --- |
| `README.md` | 简介 / 本书导读（谁适合读、怎么读、Go→Rust 心智图） | 重写自现 README |
| `start/toolchain.md` | 环境与工具链：rustup / cargo / 第一个程序 / 项目与工作空间结构 | 【新增】★ |
| `start/go-vs-rust.md` | Go → Rust 语言对照总览（心智映射 + 差异地图） | 【新增】★（吸收现 README 对照表 + 99 精华） |
| `start/syntax-primer.md` | Rust 语法底座（宏 / Option / Result / 闭包 速览） | ← 00 |

## 语言地基 `lang/`

| 文件 | 章节 | 来源 |
| --- | --- | --- |
| `lang/ownership.md` | 所有权、借用与生命周期 | ← 00d |
| `lang/types-traits.md` | 类型系统与 trait（enum、静态 vs 动态分发） | ← 00e |
| `lang/pattern-matching.md` | 模式匹配与枚举（match/if let/解构/守卫） | 【新增】B4 |
| `lang/generics.md` | 泛型与 trait bound（where/关联类型/impl Trait/dyn） | 【新增】B5 |
| `lang/collections.md` | 集合、迭代器与字符串 | ← 00f |
| `lang/smart-pointers.md` | 智能指针全家桶（Box/Rc/RefCell/Weak/Cow） | ← 00g |
| `lang/error-handling.md` | 通用错误处理（thiserror / anyhow / panic 边界） | ← 00h |
| `lang/modules.md` | 模块、crate 与可见性 —— Go 包模型对照 | 【新增】★ |

## 并发基础 `concurrency/`

| 文件 | 章节 | 来源 |
| --- | --- | --- |
| `concurrency/os-basics.md` | 已迁入 `docs/os/*`（本路径为跳转页） | ← 00a |
| `concurrency/threads.md` | Rust 多线程与并发（thread/Arc+Mutex/mpsc/scope） | ← 00b |
| `concurrency/go-gmp.md` | Go 并发实现（GMP）参照系 | ← 00c |

## 异步主线 `async/`

| 文件 | 章节 | 来源 |
| --- | --- | --- |
| `async/basics.md` | async/await、Future 惰性、并发组合 | ← 01 |
| `async/tokio.md` | Tokio 运行时：spawn / 定时器 / 通道 / select | ← 02 |
| `async/shared-state.md` | 共享状态：Arc / RwLock | ← 03 |
| `async/task-control.md` | 超时 / Semaphore 限流 / JoinSet / try_join! | ← 09 |
| `async/notify-watch.md` | watch 热更新 / broadcast / Notify / OnceCell | ← 10 |

## HTTP 服务 `http/`

| 文件 | 章节 | 来源 |
| --- | --- | --- |
| `http/http-from-scratch.md` | 从零手写 HTTP（TCP→线程→tokio 任务，四步走） | ← 04 |
| `http/axum.md` | axum 保姆级入门（Router/提取器/响应 + Gin↔axum 速查） | ← 04a |
| `http/rest.md` | 读写接口、路径参数、错误处理 | ← 05 |
| `http/arcswap.md` | ArcSwap 把读做成完全无锁 | ← 06 |
| `http/middleware-shutdown.md` | 中间件日志 + 优雅退出 + 跨 crate 复用 | ← 07 |
| `http/redis.md` | 接入 Redis 异步读写 + 自定义错误 | ← 08 |
| `http/sqlx.md` | sqlx 数据库：PgPool/FromRow/事务/thiserror 实战 | ← 13 |

## 工程实践 `engineering/`

| 文件 | 章节 | 来源 |
| --- | --- | --- |
| `engineering/code-quality.md` | 代码规范与最佳实践：命名、模块解耦、常见方案、开源项目导读 | 【新增】 |
| `engineering/testing.md` | 测试：单元/集成/doc-test、#[tokio::test]、时间快进、oneshot | ← 11 |
| `engineering/tracing.md` | tracing 结构化日志：event/span/#[instrument]/EnvFilter | ← 12 |
| `engineering/cargo-ecosystem.md` | Cargo 生态与依赖管理：features/cargo add/常用 crate 地图 | 【新增】B6 |
| `engineering/build-deploy.md` | 构建与部署：release 调优 / musl 静态 / Docker 多阶段 | ← 14 |

## 附录 `appendix/`

| 文件 | 章节 | 来源 |
| --- | --- | --- |
| `appendix/go-rust-dict.md` | Go → Rust 翻译词典 | ← 99 |

**总计**：现有 25 篇全部保留并扩写，新增 6 篇（A 组 3 + B4/B5/B6），全书 31 章。

----

# 四、新增章节（已确认，共 6 篇 · 占位已建）

**A 组（3 篇）**

1. `docs/start/toolchain.md` —— rustup/cargo/`cargo new`/`cargo run`、工作空间、`Cargo.toml` 剖析、常用命令。对照 `go mod` / `go build` / `go run`。
2. `docs/lang/modules.md` —— `mod`/`pub`/`use`/crate/工作空间可见性，对照 Go 的 package/大小写导出/`internal`。
3. `docs/start/go-vs-rust.md` —— 值语义/错误处理/并发模型/生态选型的差异地图，作为全书索引页。

**B 组（已选 B4/B5/B6）**

4. `docs/lang/pattern-matching.md` —— 模式匹配与枚举专章（`match`/`if let`/`while let`/解构/守卫）。
5. `docs/lang/generics.md` —— 泛型与 trait bound 专章（`where`、关联类型、`impl Trait`、`dyn`）。
6. `docs/engineering/cargo-ecosystem.md` —— 依赖管理与常用 crate 生态（features、`cargo add`、版本、常用库地图）。

> B7（`http/config-and-layout.md` 服务端项目骨架）本轮未选，如后续想要可再加。

----

# 五、写作规范（硬约束，新旧文档一视同仁）

批次 2–5 每篇（含改写现有文档）都必须满足下面四条 **强制规范**：

**规范 1 · 多个一级标题**
每篇拆成若干 `#` 一级标题，用 `----` 分隔； **不再是「1 个 H1 + 一堆 H2/H3」**。二级/三级标题只在一级标题内部细分时才用。照 cpp-book 的分节方式。

**规范 2 · cpp-book 标题风格 + 长度限制**
标题 **不带序号**、简洁准确、用 **陈述句**（如「必须 join 或 detach」而非「join/detach」）；标题下可紧跟一段 `>` 引用块，用大白话、生动地把这一节要干嘛先说清。 **长度硬限制：单个标题 ≤10 个汉字（英文/符号另算，总长不超过 15）** ——如「读懂 Cargo.toml」「工作空间管多个 crate」。

**规范 3 · 每篇都配丰富示例代码**
每篇都要有对应可运行示例， **场景越丰富越好**（happy path → 边界 → 踩坑，一个概念尽量给多个由浅入深的片段）；代码里 **不吝啬【保姆级】注释** ——关键行逐行讲清「这行在干嘛、为什么这么写、Go 里对应什么」。正文片段尽量能在 `code/` 找到可跑完整版；必要时在对应 crate 增补 `bin`/`example`（源码只增不减，遵循「不内联单测、示例用 `logln!`」）。

**规范 4 · 全篇【保姆级】，不怕啰嗦**
面向「Rust 只有模糊印象」的 Go 开发者：每个新概念都掰开揉碎，宁可多解释一句，不留跳跃。默认读者不知道任何 Rust 专有名词，第一次出现就解释。

**规范 5 · 正文单行 ≤80 字**
正文中的单行文本尽量不超过 80 个字（**标题、表格、代码块不受此限** ——代码交给 rustfmt）。超长就用 **markdown 强制折行**（行尾两个空格 → 渲染成 `<br>`，两行紧挨、非空行），或改 **分点分条**。折行只在中文标点（。，；、：）之后断， **绝不在链接 `[..](..)` 或行内代码 `` `..` `` 中间断**。此规范收紧了早先「≤180、不硬折行」的通用偏好，本书按此执行。

**在满足以上四条的前提下，继续保留并强化这些既有招式：**

- **Go 对照块**：每个新概念给「你在 Go 里怎么写 → Rust 里怎么写」的并排代码。
- **踩坑 / 报错还原**：故意写错，贴编译器/运行期报错，再讲怎么读、怎么修（延续 axum 章的「报错排查」风格）。
- **🔬 底层视角**：把结论钉到 `docs/os/` 操作系统组的事实上。
- **生词表**：每篇末尾「附：本章生词表」按出现顺序解释新面孔。

----

# 六、分批推进计划（确认后执行）

每一批结束我停下来交付、你验收，再继续下一批。

- **批次 0 ✅ 已完成**：docsify 骨架 + 大纲（本文）。
- **批次 1 ✅ 已完成**：文档迁入 `docs/` 主题子目录 + 6 篇新章占位 + 最终 `_sidebar.md` + 重写 `README.md` 首页 + 跨章/源码链接修正。
- **批次 2 ✅ 已完成**：开始 + 语言地基（`docs/start/*`、`docs/lang/*`，含新增 toolchain / go-vs-rust / modules / pattern-matching / generics）。
- **批次 3 ✅ 已完成**：并发基础 + 异步主线（`docs/concurrency/*`、`docs/async/*`）。
- **批次 4 ✅ 已完成**：HTTP 服务（`docs/http/*`）。
- **批次 5 ✅ 已完成**：工程实践 + 附录（`docs/engineering/*`、`docs/appendix/*`，含新增 cargo-ecosystem）。

**全书 31 章正文全部完成**：多一级标题 + 陈述句短标题（≤10 汉字）+ 保姆级 + ≤80 折行 + 丰富保姆注释代码；旧课号引用全部改为章节链接；全量校验 **0 断链、0 空续行、0 残留旧课号**。`code/` 新增 `start-toolchain`、`lang-lab`（9 个可运行 bin）两个 crate，并给工作空间加了 `anyhow` 依赖。

> 每批「填正文」都遵守：文档全中文、Go/Gin 视角、行宽 ≤180 且不硬折行、源码只增不减、示例用 `logln!`、本机不跑 cargo（Linux 侧编译验证）。

----

# 七、决策记录（已确认）

1. **覆盖广度**：以服务端 / async 为主线，适度扩边。
2. **落地方式**：原地改造 async-lab（git 根 `D:/code/lab`，`master` 分支；本会话只移动/新增文件，不提交）。
3. **目录位置**：主题目录收进 `docs/` 下。
4. **编号**：侧栏不带编号（照 cpp-book）。
5. **新增章节**：A 组 3 篇 + B4 + B5 + B6，共 6 篇。
6. **推进**：先骨架 + 大纲，确认后逐章填；每批交付验收。

> 如需调整分组/命名/顺序（例如把 `http/redis.md` + `http/sqlx.md` 拆成独立「数据接入」组），随时说，越早越省返工。

----

# 八、扩章与拆分（批次 6 · 进行中）

> 2026-07-16 确认：在已完成 31 章基础上，语言地基加深 + HTTP 增 serde 章；  
> **分批验收**；「函数与闭包」放在「所有权与借用」之后。

## 硬约定 · 一章一 crate

- 目录名 = `docs` 子目录 + `-` + md 文件名（无后缀），例：`docs/lang/basics.md` → `code/lang-basics/`。
- 每篇文档 **开头引用块** 必须标明：代码路径 + `cargo run -p <包名>`（在 `code/` 下执行）。
- 语言地基新/拆章一律走此约定；异步/HTTP 旧编号 crate（`01-…`）可保留。
- 旧聚合 crate `lang-lab`：**批次 G 已删除**；示例已全部迁入各章独立 crate。

## 语言地基最终顺序（14 章）

1. `basics` 基础类型【新·批次 B】  
2. `control-flow` 流程控制【新·批次 B】  
3. `ownership` 所有权与借用【拆瘦·批次 C】  
4. `functions-closures` 函数与闭包【新·批次 B】  
5. `lifetimes` 生命周期【拆出扩写·批次 C】  
6. `types-traits`（保留）  
7. `pattern-matching`（保留）  
8. `strings-slices` 字符串、数组与切片【拆·批次 D】  
9. `collections` 集合：Vec 与 HashMap【拆后聚焦·批次 D】  
10. `iterators` 迭代器【拆·批次 D】  
11. `generics`（保留）  
12. `smart-pointers`【大扩写·批次 E】  
13. `error-handling`（保留）  
14. `modules`（保留）

HTTP 新增：`http/serde-json.md`（插在 axum 与 rest 之间）【批次 F】。

## 分批进度

| 批次 | 内容 | 状态 |
| --- | --- | --- |
| **A** | 侧栏/README/PLAN + 占位 md + 全部 `lang-*` / `http-serde-json` crate 骨架 | ✅ |
| **B** | 基础类型 + 流程控制 + 函数与闭包（正文 + 示例 crate） | ✅ |
| **C** | 拆所有权 / 生命周期（正文 + 示例 crate） | ✅ |
| **D** | 拆 collections 三章（正文 + 示例 crate） | ✅ |
| **E** | 智能指针保姆级扩写 | ✅ |
| **F** | HTTP serde-json 专章 | ✅ |
| **G** | 交叉链接 / syntax-primer 收束 / 移除 lang-lab | ✅ |

写作规范仍遵守本文「五、写作规范」；示例一律 `logln!`，场景尽量丰富。

**批次 G 交付摘要（2026-07-16）**：

- 迁移 `types`/`pattern`/`generics`/`error`/`modules`/`threads` → 对应 `lang-*` / `concurrency-threads`
- 删除 `code/lang-lab/`，工作空间 members 已更新
- `syntax-primer` / `README` / 各章文首去掉「过渡期 lang-lab」说法
- 全书扩章拆章路线 A–G 全部完成

----

# 九、面向 login 补课

> 目标：让没有 Rust 基础的读者在接手 `message-service` 的 `login` 服务前，
> 能先用最小示例补齐真实代码依赖的知识，不修改 `login` 本身。

| 批次 | 内容 | 状态 |
| --- | --- | --- |
| **1** | UDP/IPv6 双栈、生产任务生命周期、reqwest 上游容错 | ✅ |
| **2** | Redis Cluster、真实网络集成测试与故障注入 | ✅ |
| **3** | 配置/CLI/平台差异、日志滚动与 Criterion | 待开始 |
| **4** | 部署接手清单与 login 串讲 | 待开始 |

每批仍遵守“一章一 crate”、保姆级注释、Go 对照、常见坑、动手实验和生词表约定。

----

# 十、网络编程扩章（批次 N · 正文已齐）

> 目标：把「网络编程」组从单章 UDP 扩成面向网络小白业务程序员的保姆级系列。  
> 决策：核心 12 章 + 进阶 3 章；MQTT/gRPC 含 message-service 脱敏阅读指引；  
> 不修改 login / message-service 源码；示例一律脱敏（`127.0.0.1` / `example.com`）。

## 章节清单（`docs/network/`）

| 文件 | crate | 说明 |
| --- | --- | --- |
| `layers.md` | `network-layers` | 分层与模型 |
| `addressing.md` | `network-addressing` | IP、端口、DNS |
| `socket.md` | `network-socket` | Socket 详解 |
| `tcp.md` | `network-tcp` | TCP 保姆级 |
| `udp-sockets.md` | `network-udp-sockets` | 已有，补交叉链接 |
| `http-protocol.md` | `network-http-protocol` | HTTP 协议入门 |
| `tls.md` | `network-tls` | TLS 与 HTTPS |
| `websocket.md` | `network-websocket` | WebSocket |
| `mqtt.md` | `network-mqtt` | MQTT + 消息服务阅读指引 |
| `rpc-grpc.md` | `network-rpc-grpc` | gRPC + 消息服务阅读指引 |
| `timeouts-retries.md` | `network-timeouts-retries` | 超时、重试、连接池 |
| `debug-tools.md` | `network-debug-tools` | 抓包与排障 |
| `load-balancing.md` | `network-load-balancing` | 负载均衡直觉 |
| `proxy-nat.md` | `network-proxy-nat` | 代理与 NAT |
| `quic-http3.md` | `network-quic-http3` | QUIC 与 HTTP/3 |

## 进度

| 批次 | 内容 | 状态 |
| --- | --- | --- |
| **N1** | 分层 / 寻址 / Socket / TCP；UDP 交叉链接 | ✅ |
| **N2** | HTTP 协议 / TLS / 超时重试 / 排障 | ✅ |
| **N3** | WebSocket / MQTT / gRPC | ✅ |
| **N4** | 负载均衡 / 代理 NAT / QUIC | ✅ |
| **导航** | `_sidebar.md` / `README.md` / 本表 | ✅ |

阅读顺序：操作系统 → 并发基础 → 网络编程（侧栏自上而下）→ 异步主线 → HTTP 服务。

----

# 十一、操作系统栏目扩章（批次 O · 正文已齐）

> 目标：在「网络编程」前新增同级「操作系统」组；拆扩写原 `concurrency/os-basics.md`。  
> 已确认：侧栏顺序为 语言地基 → 操作系统 → 并发 → 网络；信号/时钟/容器三章一并写入。

## 章节清单（`docs/os/`）

| 文件 | crate |
| --- | --- |
| `computer-basics.md` | `os-computer-basics` |
| `cpu-memory.md` | `os-cpu-memory` |
| `disk-io.md` | `os-disk-io` |
| `process-thread.md` | `os-process-thread` |
| `user-kernel.md` | `os-user-kernel` |
| `scheduling.md` | `os-scheduling` |
| `coroutine-state.md` | `os-coroutine-state` |
| `virtual-memory.md` | `os-virtual-memory` |
| `file-fd.md` | `os-file-fd` |
| `blocking-io.md` | `os-blocking-io` |
| `sync-primitives.md` | `os-sync-primitives` |
| `perf-cost.md` | `os-perf-cost` |
| `signals-lifecycle.md` | `os-signals-lifecycle` |
| `time-clock.md` | `os-time-clock` |
| `cgroup-container.md` | `os-cgroup-container` |

`docs/concurrency/os-basics.md` 已改为迁移跳转页。网络组章节未搬迁，仅加强交叉链接。

| 批次 | 内容 | 状态 |
| --- | --- | --- |
| **O1** | 组成 / CPU 内存 / 磁盘 / 进程线程 | ✅ |
| **O2** | 用户态 / 调度 / 协程状态机 / 虚拟内存 | ✅ |
| **O3** | fd / 阻塞多路复用 / 锁 / 性能成本 | ✅ |
| **O4** | 信号 / 时钟 / 容器 + 导航与链接扫尾 | ✅ |
