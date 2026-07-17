# JSON 是服务端日常

> 代码：[`code/http-serde-json/`](../../code/http-serde-json/)　  
> 运行：在 `code/` 目录下执行 `cargo run -p http-serde-json`

> 前置：[《axum 入门》](axum.md)（知道有 `Json<T>` 即可）。  
> Go 里你用 `encoding/json` + struct tag；Rust 里标准答案是 **`serde` + `serde_json`**。  
> 这一章把派生宏、往返、字段属性、`Option`/`null`、枚举标签讲透——  
> [《读写接口与错误处理》](rest.md) 里的请求/响应体，全靠这套。

本课 **不启动 HTTP 端口**：先把序列化本身练熟。  
axum 的 `Json<T>` 只是在 HTTP 边界帮你调用同一套 API。

----

# serde 是一套协议

> 先分清两个 crate，别混：

| crate | 干什么 | Go 对照 |
| --- | --- | --- |
| **serde** | 「可序列化」的 **trait + 派生宏**（格式无关） | 有点像「实现了某种编解码接口」 |
| **serde_json** | JSON 这种 **具体格式** 的读写 | `encoding/json` |

还有 `serde_yaml`、`toml` 等——同一套 `Serialize`/`Deserialize`，换格式 crate 即可。  
服务端日常 95% 时间是 JSON。

启用派生（本工作区已开）：

```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

----

# 打上派生就能往返

> 对照 Go：给字段加 `json:"..."`，再 `Marshal` / `Unmarshal`。  
> Rust：结构体上 `#[derive(Serialize, Deserialize)]`，再 `to_string` / `from_str`。

```rust
#[derive(Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    active: bool,
}

let s = serde_json::to_string(&u)?;          // ≈ json.Marshal
let u: User = serde_json::from_str(&s)?;     // ≈ json.Unmarshal
let bytes = serde_json::to_vec(&u)?;         // 字节版，贴合 HTTP body
let u: User = serde_json::from_slice(&bytes)?;
```

常用 API：

| 函数 | 方向 | 备注 |
| --- | --- | --- |
| `to_string` / `to_vec` | 结构体 → JSON | 紧凑 |
| `to_string_pretty` | 结构体 → JSON | 缩进，调试用 |
| `from_str` / `from_slice` | JSON → 结构体 | 类型写左边或 turbofish |
| `to_writer` / `from_reader` | 流式 | 大文件、连接 |

字段默认名 = Rust 字段名（通常 `snake_case`）。  
要对齐前端的 `camelCase`，见下文 `rename_all`。

----

# Value 与 json!

> 有时你不想先定义结构体——动态拼一段 JSON，或只摸几个字段。  
> 用 `serde_json::Value`（对照 Go 的 `map[string]any` / `json.RawMessage` 部分场景）。

```rust
use serde_json::{json, Value};

let v: Value = json!({
    "id": 7,
    "tags": ["rust", "go"],
});
println!("{}", v["id"]);                 // 索引，缺了得 Null
v.get("tags").and_then(|t| t.get(0));  // 安全取值 → Option
```

`json!` 是构造 `Value` 的宏，写起来像字面量。  
也可以 `from_value` 把 `Value` 再转成强类型结构体（只取你关心的字段）。

服务端接口 **尽量用强类型**；`Value` 适合网关透传、不稳定的外部 JSON、快速脚本。

----

# 改名：rename 与 rename_all

> Go：`UserID int64 \`json:"userId"\``。  
> Rust：属性写在字段或结构体上。

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiUser {
    user_id: u64,
    display_name: String,
    #[serde(rename = "isAdmin")]  // 单个覆盖
    admin: bool,
}
```

常见 `rename_all`：`camelCase`、`snake_case`、`PascalCase`、`SCREAMING_SNAKE_CASE`。  
序列化出去、反序列化进来都认这个名字（除非你用更细的 `rename` / `alias`）。

----

# Option 缺字段与 null

> 这是 Go 转 Rust 写 API 时最容易懵的点。  
> Rust 没有「零值冒充没传」——用 `Option` 表达「可能没有」。

```rust
#[derive(Deserialize)]
struct Patch {
    title: Option<String>,
    views: Option<u64>,
}
```

反序列化约定（默认）：

| JSON | 字段类型 `Option<T>` |
| --- | --- |
| `"title":"hi"` | `Some("hi")` |
| `"title":null` | `None` |
| 字段根本不出现 | `None` |

注意：默认情况下 **「缺字段」和「显式 null」都进 `None`**，分不开。  
多数 PATCH 够用；若必须区分「没传」vs「传了 null 表示清空」，  
要上 `#[serde(default)]` + 包装类型或 `serde_with` 等进阶技巧（需要时再查）。

序列化时：`None` **默认输出 `null`**（键还在）。  
不想输出这个键：

```rust
#[serde(skip_serializing_if = "Option::is_none")]
note: Option<String>,
```

----

# default 与 skip

> 缺字段时给默认值；或某些字段根本不进 JSON。

```rust
#[derive(Serialize, Deserialize)]
struct Config {
    #[serde(default)]                      // 缺 → 0
    retries: u32,
    #[serde(default = "default_timeout")]  // 缺 → 调用函数
    timeout_ms: u64,
    #[serde(skip)]                         // 序列化+反序列化都忽略
    cache_hit: bool,
}

fn default_timeout() -> u64 { 3000 }
```

| 属性 | 作用 |
| --- | --- |
| `default` | 反序列化缺字段时用 `Default::default()` |
| `default = "path"` | 缺字段时调用函数 `path()` |
| `skip` | 双向忽略（内部状态） |
| `skip_serializing` | 只出站忽略 |
| `skip_deserializing` | 只入站忽略 |
| `skip_serializing_if = "pred"` | 谓词为真则不输出该字段 |

----

# flatten 摊平嵌套

> 想把嵌套结构体的字段「摊」到同一层 JSON——查询参数、兼容老 API 时常用。

```rust
#[derive(Deserialize)]
struct Page { limit: u32, offset: u32 }

#[derive(Deserialize)]
struct Search {
    q: String,
    #[serde(flatten)]
    page: Page,
}
// JSON: {"q":"rust","limit":10,"offset":0}
// 而不是 {"q":"rust","page":{"limit":10,"offset":0}}
```

----

# 枚举怎么变成 JSON

> Go 没有真正的求和类型，枚举 JSON 往往手写。  
> serde 给枚举准备了几种「标签风格」（tagging）。

**默认：外部标签（externally tagged）**

```json
{"Failed":{"reason":"timeout"}}
```

**邻接标签（adjacently tagged）** ——API 里很常见：

```rust
#[serde(tag = "type", content = "data")]
enum Event {
    Login { user: String },
    Logout { user: String },
    Ping,
}
// {"type":"Login","data":{"user":"alice"}}
```

还有 `#[serde(tag = "type")]` 内部标签、`#[serde(untagged)]` 无标签（靠形状猜，脆弱，慎用）。

和前端约定好一种风格，全项目统一；不要混用。

----

# 反序列化错误值得读

> `from_str` 失败时返回的 `Error` **带路径**，比 Go 的许多报错好读：

```text
invalid type: string "not-a-number", expected u64 at line 1 column 27
missing field `name`
```

写校验逻辑时：能靠类型/属性在 serde 层拦住的，就别进业务再手写。  
需要「字段在，但业务不合法」（如 age < 0）→ 反序列化后再校验，或自定义 `Deserialize`。

可选严格模式：结构体上 `#[serde(deny_unknown_fields)]`，  
多传未知字段直接失败（默认是忽略未知字段——和 Go 默认类似）。

----

# 和 axum Json 的关系

> 心智模型就两行：

```
请求：HTTP body 字节 ──from_slice──► T: Deserialize
响应：T: Serialize ──to_vec──► HTTP body 字节
```

handler 里：

```rust
async fn create(Json(body): Json<NewItem>) -> Json<Resp> { ... }
```

等价于框架帮你：

1. 读 body；  
2. `serde_json::from_slice`；失败 → 400/422， **不进 handler**；  
3. 返回时把 `Resp` `to_vec` 写回，并带上 `Content-Type: application/json`。

对照 Gin：`c.ShouldBindJSON(&body)` + `c.JSON(200, resp)`。  
下一课 [《读写接口与错误处理》](rest.md) 把它接到路径参数、状态码、`Result` 错误上。

工作区给 serde 开了 `rc` 特性时，还可以直接 `Serialize` 某些 `Arc<T>`——  
见 [《ArcSwap 无锁读》](arcswap.md)。

----

# Go 对照速查

| Go | Rust |
| --- | --- |
| `json.Marshal(v)` | `serde_json::to_vec(&v)` / `to_string` |
| `json.Unmarshal(b, &v)` | `serde_json::from_slice(b)` / `from_str` |
| `` json:"userId" `` | `#[serde(rename = "userId")]` |
| 全结构 camelCase | `#[serde(rename_all = "camelCase")]` |
| `omitempty` | `skip_serializing_if = "Option::is_none"` 等 |
| `json:"-"` | `#[serde(skip)]` |
| `map[string]any` | `serde_json::Value` |
| 指针字段表示可选 | `Option<T>` |
| `UseNumber` 等 Decoder 选项 | `Deserializer` / 特征开关（进阶） |

----

# 动手实验

```bash
cd code
cargo run -p http-serde-json
```

1. 给 `User` 加 `#[serde(rename_all = "camelCase")]`，看输出变化；  
2. 用错误类型的 JSON 调 `from_str`，读完整错误字符串；  
3. 给结构体加 `deny_unknown_fields`，多传一个字段看是否失败；  
4. 把 `Event` 改成默认外部标签，对比 JSON 形状；  
5. 打开 [《读写接口与错误处理》](rest.md) 的 `NewItem`，对照它如何 `Deserialize`。

----

# 三句话带走

1. **`Serialize` / `Deserialize` + `serde_json`** = Rust 版 `encoding/json`。  
2. **命名用 `rename`/`rename_all`，可选用 `Option`，缺省用 `default`，出站省略用 `skip_serializing_if`。**  
3. **`Json<T>` 只是 HTTP 边界的糖** ——学会本课，axum/reqwest/配置文件全都通。

下一章：[《读写接口与错误处理》](rest.md)。

----

# 附：本章生词表

- **serde**：序列化框架（trait + derive），与具体格式无关。  
- **serde_json**：JSON 格式的 serde 实现。  
- **`Serialize` / `Deserialize`**：出站 / 入站 trait；通常派生。  
- **`Value` / `json!`**：动态 JSON 树与字面量宏。  
- **`rename` / `rename_all`**：字段或全体命名策略。  
- **`default` / `skip` / `skip_serializing_if`**：缺省、忽略、条件省略。  
- **`flatten`**：嵌套结构体字段摊平到同一层。  
- **枚举 tagging**：外部 / 内部 / 邻接 / 无标签等 JSON 形状。  
- **`deny_unknown_fields`**：禁止未知字段。  
- **`Json<T>`（axum）**：提取器/响应包装，内部调用 serde_json。
