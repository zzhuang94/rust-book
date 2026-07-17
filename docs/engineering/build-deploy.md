# 单二进制爽感

> 纯文档课（配置与命令为主）。以部署 `http-http-from-scratch` 为例，所有片段可直接套用到任何一课。

回答 Go 程序员必问的三件事：release 怎么调优、怎么交叉编译出「扔哪都能跑」的静态二进制、Docker 镜像怎么做小。  
目标是把 Go 的「单二进制爽感」带到 Rust。

----

# debug 与 release

> 上线前先搞清你在跑哪个——这直接解释「Rust 怎么比 Go 还慢」的误会。

```bash
cargo build              # debug：编译快、不优化、带调试信息 → target/debug/
cargo build --release    # release：全量优化 → target/release/
cargo run --release -p http-http-from-scratch
```

差距有多大：debug 构建的 Rust 程序常比 release **慢 10~50 倍**（Go 没这个分裂——go build 默认就带优化）。  
两条纪律：**压测和上线永远用 release**；平时开发用 debug（编译快）。这也解释了为什么有人抱怨「Rust 怎么比 Go 还慢」——多半在跑 debug 构建。

----

# release 四个旋钮

> `Cargo.toml` 里的 `[profile.release]` 有四个常用旋钮。

```toml
# 工作区根 Cargo.toml
[profile.release]
opt-level = 3        # 优化级别：3=速度优先（默认）；"s"/"z"=体积优先
lto = "thin"         # 链接时优化：跨 crate 内联，快一点小一点（"fat" 更狠但编译慢）
codegen-units = 1    # 代码生成单元数：1=优化最充分（编译更慢）；默认 16 偏编译速度
strip = true         # 剥掉符号表和调试信息：二进制立减几 MB（≈ go 的 -ldflags "-s -w"）
```

再狠一点的可选项：`panic = "abort"`（panic 直接终止不展开栈，更小更快；代价是 catch_unwind 失效——用了它就别依赖 [《通用错误处理》](../lang/error-handling.md) 的边界接 panic）。

体积预期管理：hello world 级 release+strip 约几百 KB～1MB；带 tokio+axum 的服务几 MB——和 Go 同量级（Go 二进制自带运行时也是几 MB 起步）。  
**Rust 二进制默认也是静态链接所有 Rust 代码的**，唯一的动态依赖通常只剩 libc——这正是下一节的主角。

----

# 交叉编译静态二进制

> 复刻 `GOOS=linux GOARCH=amd64` 一行出静态二进制的爽感。Rust 的对应物分两层。

**rustup target：标准姿势**

```bash
rustup target add x86_64-unknown-linux-musl     # 装目标平台的标准库
cargo build --release --target x86_64-unknown-linux-musl
# 产物：target/x86_64-unknown-linux-musl/release/http-http-from-scratch —— 100% 静态，Alpine/scratch 直接跑
```

target 三元组认识几个常用的：`x86_64-unknown-linux-gnu`（常规 Linux，动态链 glibc）、  
`x86_64-unknown-linux-musl`（**静态链 musl，"扔哪都能跑"**，≈ CGO_ENABLED=0 的效果）、  
`aarch64-unknown-linux-musl`（ARM 服务器/树莓派）、`x86_64-pc-windows-msvc`。

**现实的坑与两个工具**：纯 Rust 项目 musl 目标通常直接成功；但只要依赖里有 C 代码（openssl 最典型），  
跨平台就需要目标平台的 C 交叉工具链——这是 Rust 交叉编译不如 Go 丝滑的根源（Go 用纯 Go 实现了几乎一切，  
Rust 生态复用了大量 C 库）。两个救星：

```bash
# 方案一：cross —— 在 Docker 容器里替你备好全套工具链，命令同 cargo
cargo install cross
cross build --release --target aarch64-unknown-linux-musl

# 方案二：cargo-zigbuild —— 用 zig 当万能 C 交叉编译器，不要 Docker
cargo install cargo-zigbuild   # 另需安装 zig
cargo zigbuild --release --target x86_64-unknown-linux-musl
```

配套心法：**优先选纯 Rust 依赖** 规避 C——本教程已经这么做了（[《接入 Redis》](../http/redis.md) 没开 TLS；  
要 TLS 时选 rustls 而不是 openssl， [《sqlx 数据库》](../http/sqlx.md) 同理有 `tls-rustls` 特性）。

----

# Docker 多阶段

> 多阶段构建 + 依赖缓存层是 Rust 镜像的命根子（Rust 编译慢，缓存必须做对）。

```dockerfile
# ---------- 构建阶段 ----------
FROM rust:1.85-slim AS builder
WORKDIR /app

# 第一层：只拷贝清单文件，用一个假 main 把【依赖】编译出来 —— 这层只要 Cargo.toml 不变就永远命中缓存
COPY Cargo.toml Cargo.lock ./
COPY */Cargo.toml ./placeholder/            # 工作区各成员的清单（按实际目录结构调整）
RUN mkdir -p src && echo "fn main() {}" > src/main.rs && \
    cargo build --release -p http-http-from-scratch || true

# 第二层：拷真代码，只需增量编译业务部分
COPY . .
RUN cargo build --release -p http-http-from-scratch

# ---------- 运行阶段：只带二进制 ----------
FROM debian:bookworm-slim
RUN useradd -r app
COPY --from=builder /app/target/release/http-http-from-scratch /usr/local/bin/http-http-from-scratch
USER app
EXPOSE 7080
CMD ["http-http-from-scratch"]
```

要点逐条：

- **多阶段**：构建镜像 1GB+，运行镜像只拷一个二进制进 slim（几十 MB）——和 Go 的多阶段构建同款套路；
- **依赖缓存层** 是 Rust Docker 构建的命根子（上面用「假 main」手法；更工程化的用 [cargo-chef](https://github.com/LukeMathWalker/cargo-chef)，  
  原理相同做得更对）；
- 想要极限小镜像：上一节的 musl 静态二进制 + `FROM scratch`/`FROM gcr.io/distroless/static`—— **这就是 Go 用户熟悉的 scratch 镜像玩法**，  
  前提是全静态（musl + rustls）；
- 别忘了 `.dockerignore`：至少排除 `target/`（几个 GB 的构建缓存拷进 Docker 上下文，谁传谁哭）。

----

# 运行期配置清单

> 上线前把前面各课的伏笔串起来，逐项打勾。

| 项 | 做法 | 出处 |
| --- | --- | --- |
| 配置 | 环境变量读取（`std::env::var` + 默认值），复杂了用 figment/config crate | [《接入 Redis》](../http/redis.md) / [《sqlx 数据库》](../http/sqlx.md) 的 DATABASE_URL 模式 |
| 日志 | tracing + `.json()` 输出，RUST_LOG 控制级别 | [《tracing 结构化日志》](tracing.md) |
| 优雅退出 | `with_graceful_shutdown` + 同时等 SIGTERM（容器停止发的就是它！） | [《中间件与优雅退出》](../http/middleware-shutdown.md) |
| 后台任务收尾 | CancellationToken 广播 | [《Tokio 运行时》](../async/tokio.md) |
| 健康检查 | `/health` 路由 + Dockerfile `HEALTHCHECK` 或 k8s probe | [《从零手写 HTTP》](../http/http-from-scratch.md) |
| worker 线程数 | 默认=核数；容器限核时注意 cgroup 感知，可用 TOKIO_WORKER_THREADS 显式指定 | [《Tokio 运行时》](../async/tokio.md) |

**systemd 一提（非容器部署）**：

```ini
[Service]
ExecStart=/usr/local/bin/http-http-from-scratch
Environment=RUST_LOG=info
Restart=on-failure
```

静态 musl 二进制 scp 上去 + 一个 unit 文件就能跑—— **Go 式部署的完整复刻**。

----

# Go↔Rust 部署对照

| | Go | Rust |
| --- | --- | --- |
| 默认优化 | go build 即优化 | 必须 `--release`（默认 debug！） |
| 减体积 | `-ldflags "-s -w"` | `strip = true` + `opt-level = "z"` |
| 静态二进制 | `CGO_ENABLED=0` | `--target *-musl`（+ 纯 Rust 依赖/rustls） |
| 交叉编译 | GOOS/GOARCH 环境变量，几乎无坑 | rustup target；有 C 依赖时用 cross/zigbuild |
| 最小镜像 | scratch + 静态二进制 | 同款（musl 版）；或 distroless/static |
| 构建缓存 | 快，不太需要操心 | 慢，Docker 里必须做依赖缓存层（cargo-chef） |
| 版本信息注入 | `-ldflags -X` | `env!("CARGO_PKG_VERSION")` / build.rs |

----

# 动手实验

1. **量化 debug vs release**：对 axum 服务分别构建后用 `ab -n 20000 -c 100` 压测对比 QPS；  
   再对比二进制大小（strip 前后各看一次）；
2. **出一个静态二进制**（Linux 侧）：`rustup target add x86_64-unknown-linux-musl` 构建 http-http-from-scratch，  
   `ldd` 验证输出 "not a dynamic executable"，拷到任一发行版容器里裸跑；
3. **写出能缓存的 Dockerfile**：按上面给 http-http-from-scratch 做镜像，改一行 handler 代码重新 build——确认依赖层显示 CACHED、  
   总时间从分钟级降到秒级；
4. **优雅退出闭环验证**：`docker stop`（发 SIGTERM）你的容器，确认日志出现 [《中间件与优雅退出》](../http/middleware-shutdown.md) 的「等待在途请求完成」而不是硬断；
5. **profile 旋钮**：分别用默认、`lto="thin"+codegen-units=1`、`opt-level="z"+strip` 构建，  
   记录（编译耗时、二进制大小、ab QPS）三元组，体会取舍。

----

# 三句话带走

1. **上线永远 `--release`**（debug 慢 10~50 倍是正常现象不是 Rust 慢）；profile 四旋钮 opt-level/lto/codegen-units/strip，  
   体积敏感再加 panic="abort"。
2. **`--target x86_64-unknown-linux-musl` = Rust 版 `CGO_ENABLED=0`**：  
   全静态、scratch 可跑；坑全在 C 依赖——选纯 Rust 库（rustls），绕不开就 cross/zigbuild。
3. **Docker 多阶段 + 依赖缓存层是 Rust 镜像的命根子**（cargo-chef）；运行期清单——env 配置、  
   tracing JSON 日志、SIGTERM 优雅退出、/health——前面各课已备齐，本课负责打包出厂。

----

# 附：本课生词表

- **`--release` / `[profile.release]`** ——优化构建及其配置节；debug 与 release 是两套产物目录。
- **`opt-level` / `lto` / `codegen-units` / `strip`** ——速度或体积 / 链接时跨 crate 优化 / 并行代码生成单元（1=最优化）/ 剥符号。
- **target 三元组** ——`架构-厂商-系统-libc`，如 `x86_64-unknown-linux-musl`；  
  `rustup target add` 安装、`--target` 使用。
- **musl vs glibc** ——静态可移植 vs 动态链接系统 libc；「扔哪都能跑」选 musl。
- **cross / cargo-zigbuild** ——容器化工具链 / zig 当交叉 C 编译器，治「C 依赖跨编译」顽疾。
- **cargo-chef** ——Docker 依赖缓存层的标准工具（「假 main」手法的工程化版）。
- **`panic = "abort"`** ——panic 不展开直接终止：更小更快，但 catch_unwind 失效。
- **`env!("CARGO_PKG_VERSION")`** ——编译期读 Cargo.toml 版本号嵌进二进制 ≈ Go 的 -ldflags -X。
