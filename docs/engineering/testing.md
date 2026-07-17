# 和 go test 对照

> 代码：`code/engineering-testing/`（纯库 crate）　运行：`cargo test -p engineering-testing`（被测对象三个模块：  
> calc 纯函数 / cache 异步 TTL 缓存 / api axum 接口，测试代码在 `tests/` 目录下三个文件）

| | Go | Rust |
| --- | --- | --- |
| 跑测试 | `go test ./...` | `cargo test`（工作区加 `-p 包名`） |
| 测试函数 | `func TestXxx(t *testing.T)` | `#[test] fn 任意名()`，panic 即失败 |
| 断言 | `if got != want { t.Errorf }` 或 testify | `assert_eq!/assert!/matches!` 内置 |
| 异步测试 | 普通函数里开 goroutine | `#[tokio::test]` |
| 接口测试 | httptest | `tower::ServiceExt::oneshot`（不开端口） |
| 文档示例可执行 | Example 函数比对输出 | **doc-test：文档里的代码块直接跑断言** |
| 测试依赖隔离 | 无（都在 go.mod） | `[dev-dependencies]` 只参与测试编译 |

最大的观念差：Rust 的失败 = panic，所以 **任何断言宏都能当 t.Fatal 用**，没有 t.Error/t.Fatal 之分（要「失败后继续」就拆成多个测试）。

----

# 测试放哪

> 两种位置，先都认识。

**(1) 内联单元测试**（Rust 社区最常见，读开源必见）：和被测代码同文件，能测私有函数：

```rust
// 某个 src/xxx.rs 的末尾（本项目不采用这种，但你必须认识它）
#[cfg(test)]              // 条件编译：只有 cargo test 才编译这一块
mod tests {
    use super::*;         // 引入本文件的所有东西（包括私有的）
    #[test]
    fn it_works() { assert_eq!(private_fn(2), 4); }
}
```

**(2) 独立集成测试**（本项目的约定）：`tests/` 目录下每个 `.rs` 文件编译成 **独立的测试程序**，  
以外部使用者身份 `use engineering_testing::...`——只能碰 pub 的东西，逼着你测公开契约而不是实现细节：

```
engineering-testing/
├── src/…            被测库
└── tests/
    ├── calc_test.rs   ← 每个文件独立编译、可并行
    ├── cache_test.rs
    └── api_test.rs
```

对照 Go：`tests/` 目录 ≈ Go 的 `package foo_test`（黑盒测试包）；内联 mod tests ≈ 同包白盒测试。

----

# 断言三件套

> 看 `tests/calc_test.rs`：三个断言宏覆盖日常。

```rust
assert_eq!(k, "name");                       // 失败时自动打印 left / right 两边的值
assert!(err.contains("缺少等号"), "实际错误: {err}");  // 布尔断言 + 自定义失败消息
assert!(matches!(parse_kv("=v"), Err(_)));   // 模式断言：值长得像不像这个 pattern
```

`matches!` 特别适合断言 enum 变体（「是 Err 就行，不关心内容」）—— [《类型系统与 trait》](../lang/types-traits.md) 的 match 在断言里的化身。

----

# doc-test 防腐

> Go 没有的彩蛋：文档里的例子会被 `cargo test` 抠出来 **编译执行**，文档示例永远不过时。

看 `src/calc.rs` 的 `parse_kv`：`///` 文档注释里那段代码块会被跑：

```rust
/// ```
/// let (k, v) = engineering_testing::calc::parse_kv("name=tokio").unwrap();
/// assert_eq!(k, "name");
/// ```
pub fn parse_kv(...)
```

跑 `cargo test -p engineering-testing` 的输出里有单独一节 `Doc-tests`。含义：**API 文档里的示例代码是被 CI 保证能跑的** ——重构改了签名，  
文档例子立刻编译失败。Go 的 Example 函数只比对 stdout，Rust 直接跑断言，强一档。

----

# tokio::test 异步

> 异步测试用 `#[tokio::test]`——给这个测试函数配一个专属 Tokio 运行时。

```rust
#[tokio::test]
async fn 缓存_写入后能读到() {
    let cache = Cache::new(Duration::from_secs(60));
    cache.set("k", "v").await;
    assert_eq!(cache.get("k").await, Some("v".to_string()));
}
```

`#[tokio::test]` = 给这个测试函数配一个专属 Tokio 运行时（≈ 每个测试自带 `#[tokio::main]`）。  
函数体就是普通 async 代码，spawn/select/通道随便用。

----

# 时间快进

> 本课最值钱的一招：测「60 秒后过期」不用真等 60 秒——让运行时接管时钟，手动快进。

```rust
#[tokio::test(start_paused = true)]        // 运行时时钟从【暂停】状态开始
async fn 缓存_到期后拿不到() {
    let cache = Cache::new(Duration::from_secs(60));
    cache.set("k", "v").await;

    tokio::time::advance(Duration::from_secs(59)).await;   // 手动快进 59 秒
    assert!(cache.get("k").await.is_some());

    tokio::time::advance(Duration::from_secs(2)).await;    // 累计 61 秒
    assert!(cache.get("k").await.is_none());               // 过期了
}
```

原理与前提：

- Tokio 的定时器/时钟全归运行时管，所以测试能整体接管时间：`start_paused` 暂停，`advance` 快进，  
  sleep/interval/Instant 全部服从；
- **前提是被测代码用 tokio 的时间**（`tokio::time::Instant`/`sleep`），`cache.rs` 特意这么选型——用 std 时间就管不到了；
- **还要开 `test-util` 特性**：`advance` / `start_paused` 都在这个 feature 里，`full` 不含它（避免生产误开时间模拟）。  
  本课在 `[dev-dependencies]` 里写 `tokio = { workspace = true, features = ["test-util"] }`，仅 `cargo test` 时合并进来；
- 对照 Go：要么真 sleep（测试慢、还容易 flaky），要么给代码注入 Clock 接口（侵入设计）。这里被测代码 **零改动**，  
  测试想快进就快进——异步定时逻辑（重试、超时、TTL）的测试从此又快又稳。

----

# oneshot 测接口

> 测 axum 接口不用开端口：Router 本质是 tower::Service，测试直接喂 Request 拿 Response，纯内存调用。

```rust
use tower::ServiceExt;        // .oneshot() 来自它（trait 方法要 use）
use http_body_util::BodyExt;  // .collect() 收响应体

let resp = app()
    .oneshot(Request::builder().uri("/add?a=1&b=2").body(Body::empty()).unwrap())
    .await.unwrap();
assert_eq!(resp.status(), StatusCode::OK);

let bytes = resp.into_body().collect().await.unwrap().to_bytes();
let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
assert_eq!(v["sum"], 3);
```

- ≈ Go 的 `httptest.NewRecorder()` + `router.ServeHTTP(w, req)`，思路一模一样；
- 可测性的关键一步在 `src/api.rs`：**把「组装 Router」抽成 `pub fn app()`**，  
  生产 main 和测试用同一个入口—— [《从零手写 HTTP》](../http/http-from-scratch.md) lib+bin 布局的又一次回报；
- 测试还顺便验证了提取器行为：`/add?a=abc` 直接断言 400（不进 handler，见 [《axum 入门》](../http/axum.md)）。

----

# 常用命令

```bash
cargo test -p engineering-testing              # 跑这个包的全部（含 doc-test）
cargo test -p engineering-testing 缓存          # 按名字过滤（≈ go test -run）
cargo test -p engineering-testing -- --nocapture   # 显示测试里的 println（默认吞掉）
cargo test -p engineering-testing -- --test-threads=1  # 串行跑（默认并行！有共享资源时注意）
cargo test --doc -p engineering-testing        # 只跑文档测试
```

⚠️ 和 Go 的一个差异要记住：**Rust 测试默认多线程并行跑**（Go 默认串行、t.Parallel 才并行）——测试之间共享文件/端口/全局状态会互相打架，  
要么加 `--test-threads=1`，要么让每个测试用独立资源。

----

# 动手实验

1. **跑通并读懂输出**：`cargo test -p engineering-testing`，数一数三类测试（单元/集成/doc-test）各在输出的哪一节；
2. **弄坏一次**：把 `parse_kv` 的 trim 删掉，看哪些测试红了、`assert_eq!` 的 left/right 怎么帮你定位；
3. **感受时间控制的价值**：把 `cache_test.rs` 的 `start_paused = true` 去掉、  
   advance 换成真 sleep——测试从毫秒级变成一分钟；改回来；
4. **加一个接口测试**：给 api.rs 加 `GET /mean?xs=1,2,3`（内部调 calc::mean），  
   先写测试再写实现——体验一把测试先行；
5. **doc-test 防腐**：把 parse_kv 返回值改成 `Option`，看文档测试立刻编译失败——「文档过时」变成编译错误。

----

# 三句话带走

1. **`#[test]` + panic 即失败**；断言三件套 assert!/assert_eq!/matches!；  
   测试位置两种——内联 `#[cfg(test)]`（能测私有，开源常见）与 `tests/` 目录（黑盒，本项目约定）；  
   doc-test 让文档示例被 CI 保证。
2. **异步测试 `#[tokio::test]`，时间敏感逻辑用 `start_paused` + `advance`** ——被测代码零改动，  
   「等一小时」毫秒跑完，前提是代码用 tokio 的时间。
3. **axum 接口用 oneshot 内存直测**（≈ httptest）：Router 抽成 `pub fn app()` 共享给生产与测试；  
   记住 Rust 测试 **默认并行**。

----

# 附：本课生词表

- **`#[test]` / `#[cfg(test)]`** ——标记测试函数 / 「只在测试编译时存在」的条件编译（内联测试模块靠它不进正式二进制）。
- **`[dev-dependencies]`** ——只有测试/示例才编译的依赖区；tower、serde_json 放这里，不污染正式构建。
- **`assert_eq!` / `matches!`** ——相等断言（失败打印两边值）/ 模式断言（配 enum 变体）。
- **doc-test** ——`///` 文档里的代码块被 cargo test 编译执行；`/// ``` ignore` 可豁免。
- **`#[tokio::test]` / `start_paused`**——测试专属运行时 / 时钟从暂停开始，  
  配 `tokio::time::advance` 快进。
- **`tower::ServiceExt::oneshot`**——把一个 Request 喂给 Service 拿 Response 的一次性调用；  
  axum 接口测试标配。
- **`http_body_util::BodyExt::collect`**——把响应体流收集成字节（`.to_bytes()`），  
  再交给 serde_json 断言。
- **`--nocapture` / `--test-threads=1`**——放行测试内打印 / 串行执行（`--` 之后的参数是传给测试程序的）。
