# 读写与错误处理

> 代码：`code/http-rest/`　运行：`cargo run -p http-rest`（本课不跑后台更新任务：  
> 数据只由 HTTP 写接口驱动，方便观察 POST 进去的数据不被覆盖）

前置：[《axum 入门》](axum.md)；JSON 字段/Option/改名等见 [《JSON 序列化与反序列化》](serde-json.md)；[《从零手写 HTTP》](http-from-scratch.md) 只有读接口。  
本课加三件事：  
1. **路径参数**（`GET /item/{idx}`）；
2. **写接口**（`POST /item`，提取 JSON 请求体）；
3. **错误处理**（`Result` 即响应，404/422 的正确姿势）。

这也是 axum「提取器」体系与「Result 即响应」最能体现设计哲学的地方。

----

# 参数类型即需求

> 再强调核心心智（本课全靠它）：**handler 的每个参数都是「提取器」，axum 按类型自动取值并转换**。

| 提取器 | 取什么 | 取失败时 | Gin 对照 |
| --- | --- | --- | --- |
| `State<T>` | 全局状态 | 不会失败（编译期绑定） | 闭包捕获 |
| `Path<T>` | 路径参数 | 类型不符 → 400 | `c.Param` + strconv |
| `Query<T>` | 查询串 | 缺字段/类型错 → 400 | `c.Query` |
| `Json<T>` | 请求体 | 解析失败 → 400/422 | `c.ShouldBindJSON` |

哲学差异：Gin 什么都从 `*gin.Context` 手动取，忘了校验就悄悄拿到零值；  
axum 在 **函数签名** 里声明要什么，框架负责搬运、校验、转换，出错以标准错误码暴露（或者干脆编译不过）。

----

# 两个 trait 定顺序

> 提取器分两类，这决定了参数怎么排。

- **`FromRequestParts`** ——只读请求「元信息」：method、URI、headers、路径参数、扩展；  
> **不碰请求体** → 一个 handler 可以有 **任意多个**，顺序随意；`State`、`Path`、`Query`、`HeaderMap` 属于此。

- **`FromRequest`** ——要 **消费整个请求体**：body 是一次性的流，读了就没了；  
> 一个 handler **最多一个**，且 **必须是最后一个参数**；`Json`、`String`、`Bytes`、`Form` 属于此。

所以 `post_item` 的签名只能这么排：

```rust
pub async fn post_item(
    State(state): State<AppState>,  // FromRequestParts，可在前
    Json(body): Json<NewItem>,      // FromRequest，必须最后
) -> ...
```

写反了会得到一条初看费解的报错：

```
error: the trait bound `...: FromRequestParts<...>` is not satisfied
```

翻译成人话：「你把消费 body 的提取器放在了非末位。」

----

# 路径参数

> `GET /item/{idx}`：路径里的一段，用 `Path<T>` 提取器接。

```rust
pub async fn get_item(
    State(state): State<AppState>,
    Path(idx): Path<usize>,
) -> Result<Json<ItemResp>, (StatusCode, String)> {
    match state.get_item(idx) {
        Some(value) => Ok(Json(ItemResp { index: idx, value })),
        None => Err((StatusCode::NOT_FOUND, format!("下标 {idx} 越界"))),
    }
}
```

路由注册：`.route("/item/{idx}", get(handler::get_item))`。

`Path<T>` 要点：`Path<usize>` 直接给你 `usize`； **解析失败自动回 400**（如 `/item/abc`），一行代码不用写；  
多个路径参数用元组 `Path((uid, pid)): Path<(u64, u64)>`，或一个 `#[derive(Deserialize)]` 结构体。

对照 Gin：

```go
r.GET("/item/:idx", func(c *gin.Context) {
    idx, err := strconv.Atoi(c.Param("idx"))
    if err != nil { c.JSON(400, ...); return }
})
```

Gin 的路径参数 **永远是字符串**，自己 strconv、自己处理失败；axum 把「解析 + 校验」这层样板整个消掉了。

----

# handler 返回 Result

> 本课重点：让 handler 返回 `Result`，成功给 200、失败给 404/422，全靠 IntoResponse。

**最省事的写法：`Result<T, (StatusCode, String)>`**  
- `Ok(Json(...))` → 200 + JSON；  
- `Err((StatusCode::NOT_FOUND, msg))` → 404 + 文本。 

能这样写的原理：axum 里 **凡实现了 `IntoResponse` 的类型都能当返回值/错误**，  
而 `Json<T>`、`(StatusCode, String)`、`Result<T, E>`（T、E 都 IntoResponse）都实现了。

**更工程化：自定义 AppError（[《接入 Redis》](redis.md) 兑现）**。项目变大后，用统一错误类型配合 `?`：

```rust
enum AppError { NotFound(String), Redis(redis::RedisError), ... }
impl IntoResponse for AppError { /* 映射状态码 + JSON 错误体 */ }
impl From<redis::RedisError> for AppError { /* 让 ? 自动转换 */ }

// handler 里：
let v = some_fallible_call().await?;  // 出错自动变 AppError → HTTP 响应
```

[《接入 Redis》](redis.md) 完整实现并讲透这套模式。本课先用简单形态建立概念。

对照 Gin 的经典 bug：

```go
if idx >= len(items) {
    c.JSON(404, gin.H{"error": "越界"})
    return   // ← 忘了这行，代码继续往下跑（著名 bug 来源）
}
```

Rust 的 `Err(...)` 一返回函数就结束—— **类型系统保证**，不存在「忘了 return 继续执行」；  
`?` 更进一步：`call()?` = 「成功取值、失败 return 这个错误」。

----

# 写接口收 JSON

> `POST /item`：用 `Json<T>` 提取器接收请求体。

```rust
#[derive(Deserialize)]              // JSON → 结构体（≈ Go json tag）
pub struct NewItem { pub value: i64 }

pub async fn post_item(
    State(state): State<AppState>,
    Json(body): Json<NewItem>,      // 最后一个参数：消费请求体
) -> (StatusCode, Json<AppendResp>) {
    let version = state.append_item(body.value);
    (StatusCode::CREATED, Json(AppendResp { version, ... }))
}
```

三个要点：

1. **`Json<T>` 必须放最后**（前面的规则），放错编译报错；
2. 客户端 JSON 缺字段/类型不符 → 提取器自动回 **422**，不进你的 handler（Gin 的 ShouldBindJSON 要自己处理 err）；
3. 返回 `(StatusCode, Json<T>)` 元组：第一个元素定状态码（这里 201 Created），第二个是 body。

写入本身走 [《共享状态：Arc / RwLock》](../async/shared-state.md) 的规矩——临界区 **不含 `.await`** ：

```rust
pub fn append_item(&self, value: i64) -> u64 {
    let mut guard = self.data.write().unwrap();  // 写锁
    guard.items.push(value);
    guard.version += 1;
    guard.version
}                                                 // guard drop，解锁
```

----

# 跑起来验证

```bash
cargo run -p http-rest
```

```bash
curl http://127.0.0.1:7080/item/0    # {"index":0,"value":10}
curl http://127.0.0.1:7080/item/99   # 404: 下标 99 越界

curl -X POST http://127.0.0.1:7080/item \
     -H 'content-type: application/json' -d '{"value": 42}'
# 201 {"version":2,"message":"已追加 42"}

curl http://127.0.0.1:7080/item/3    # ← 刚追加的 42

curl -X POST http://127.0.0.1:7080/item \
     -H 'content-type: application/json' -d '{"value": "oops"}'
# 422：类型不对，提取器拦下，没进 handler
```

----

# 动手实验

1. **验证提取器顺序**：把 post_item 参数顺序对调，`cargo build`，读懂那条 `FromRequestParts is not satisfied` 报错，  
   再改回；
2. **加查询参数**：给 get_data 加 `Query<Paging>`（`struct Paging { limit: Option<usize> }`），  
   只返回前 limit 个元素，`curl '.../data?limit=2'` 验证；
3. **触发三种错误码**：400（`/item/abc`）、404（`/item/999`）、422（POST 类型不对的 JSON），  
   理解「提取器失败」vs「handler 主动 Err」的区别。

----

# 三句话带走

1. **参数类型 = 要什么，返回类型 = 产出什么**；取错/解析失败在编译期或以标准错误码暴露。
2. 提取器分两类：`FromRequestParts`（读元信息，可多个）、`FromRequest`（**消费 body，只能一个且放最后**）。
3. handler 返回 `Result<T, E>`（E: IntoResponse）+ `?` 传播错误，比 Gin 手动 c.JSON+return 更不易出错；  
   复杂项目用自定义 AppError（[《接入 Redis》](redis.md)）。

----

# 附：本课生词表

> 通用语法见 [《Rust 语法底座》](../start/syntax-primer.md)；State/Json/serde 见 [《从零手写 HTTP》](http-from-scratch.md) 生词表。

- **`Path<T>`** ——路径参数提取器：`Path(idx): Path<usize>` 把 `{idx}` 段解析成 usize 并解构出来；  
  解析失败自动 400，不进 handler；多参数用元组或 Deserialize 结构体。
- **`Query<T>`** ——查询串提取器：`?limit=2&page=1` 解析进 `#[derive(Deserialize)]` 结构体；  
  字段用 `Option<usize>` 表示「可不传」。
- **`#[derive(Deserialize)]`** ——serde 反序列化派生：JSON/查询串 → 结构体；  
  与 `Serialize`（结构体 → JSON）方向相反；请求体结构体标它，响应体结构体标 Serialize。
- **`StatusCode`** ——http crate 的状态码类型，具名常量：`NOT_FOUND`(404)、  
  `CREATED`(201)、`INTERNAL_SERVER_ERROR`(500)…；比裸数字可读、类型安全（写不出 999）。
- **`Result<Json<T>, (StatusCode, String)>`（handler 返回类型）** ——「成功给 JSON，  
  失败给 状态码+文本」；原理：`Result`、`(StatusCode, String)`、`Json<T>` 都实现了 IntoResponse；  
  `Err(...)` 一返回函数即结束——不会「忘了 return」。
- **`(StatusCode, Json<T>)`（元组当响应）** ——第一个元素定状态码，第二个是 body；  
  不指定状态码（直接返回 `Json<T>`）默认 200。
- **`items.get(idx)` / `.copied()`** ——`.get(idx)` 是 Vec 的安全取下标，  
  返回 `Option<&T>`，越界 None（对比 `items[idx]`：越界 panic）；本课的 404 就来自这里；  
  `.copied()`：`Option<&i64>` → `Option<i64>`（仅限 Copy 类型）。
- **`Option<u64>`（请求体字段）** ——serde 约定：JSON 缺字段或 null → None；  
  有值 → Some(v)；天然表达「可传可不传」，≈ Go 里 `*uint64` 判 nil。
