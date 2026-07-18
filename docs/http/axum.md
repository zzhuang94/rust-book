# axum 怎么用

> 参考实现散布在 `code/http-http-from-scratch`、`code/http-rest`、`code/http-middleware-shutdown` 里；  
> 本篇的代码片段都可以直接放进一个 axum 0.8 项目运行（本工作区即 0.8）。

[《从零手写 HTTP》](http-from-scratch.md) 讲了「axum 在整个体系里的位置」（tokio→hyper→tower→axum），  
本篇专讲「axum 本身怎么用」——每个用法都给出 Gin 等价写法。定位是 **字典**：第一遍通读，之后写代码时随手来查。

----

# 最小应用逐行拆

> 先把两边的 hello world 并排放。

```go
// Gin
func main() {
    r := gin.Default()
    r.GET("/", func(c *gin.Context) { c.String(200, "hello") })
    r.Run(":8080")
}
```

```rust
// axum
use axum::{routing::get, Router};

#[tokio::main]                                       // (1)
async fn main() {
    let app = Router::new()                          // (2)
        .route("/", get(|| async { "hello" }));      // (3)

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();                                   // (4)
    axum::serve(listener, app).await.unwrap();       // (5)
}
```

- axum 跑在 tokio 上，所以 main 要挂 `#[tokio::main]`（Gin 不需要——Go 运行时内置，[《Tokio 运行时》](../async/tokio.md) 讲过这笔账）；
- `Router::new()` 造一个空路由表 ≈ `gin.New()`（不是 `gin.Default()`——axum 默认不带任何中间件，日志等要自己 layer）；
- `.route(路径, 方法路由)` 注册一条路由；`get(handler)` 表示「这个路径的 GET 归这个 handler」。这里 handler 是个闭包 `|| async { "hello" }`，正经项目里写具名 async fn；
- 自己 bind 端口拿 listener——axum 把「监听」和「服务」拆开了（好处：测试时可以 bind 端口 0 随机端口）；
- `axum::serve(listener, app)` 开始服务 ≈ `r.Run(":8080")`。

----

# Router 是不可变积木

> Gin 的 `r` 是个可变对象，你对着它连续 `r.GET`、`r.POST` 往里塞。  
> axum 的 Router 不一样—— **每个方法都消费旧 Router、返回一个新 Router**（所有权转移 + builder 模式）。

```rust
let app = Router::new()          // Router A
    .route("/", get(home))      // A 被吃掉，产出 Router B
    .route("/users", get(list)); // B 被吃掉，产出 Router C（可直接 serve）
```

所以 axum 路由注册总是 **一长串链式调用**；想分开写也行，但要接住返回值：

```rust
let mut app = Router::new();
app = app.route("/", get(home));       // 注意 app = ——不接住就丢了
app = app.route("/users", get(list));
```

（handler 若要用共享状态，链式末尾再加 `.with_state(...)`，见下一小节。）

## route 与 HTTP 方法

```rust
use axum::routing::{get, post, put, delete, patch};

Router::new()
    .route("/users", get(list_users).post(create_user))  // 同路径不同方法：链起来
    .route("/users/{id}", get(get_user).put(update_user).delete(delete_user))
```

| Gin | axum |
| --- | --- |
| `r.GET("/x", h)` | `.route("/x", get(h))` |
| `r.POST("/x", h)` | `.route("/x", post(h))` |
| `r.GET(...)` + `r.POST(...)` 同路径 | `.route("/x", get(h1).post(h2))` **一条 route 里链**（同路径写两条 `.route` 会 panic！这是和 Gin 的行为差异） |
| `r.Any("/x", h)` | `.route("/x", any(h))` |

## 路径参数与通配符

```rust
.route("/users/{id}", get(h))         // {id} 匹配一段 ≈ Gin 的 :id
.route("/static/{*path}", get(h))     // {*path} 匹配剩余全部 ≈ Gin 的 *path
```

- 取值不靠 `c.Param("id")`，靠 handler 参数上的 `Path` 提取器；
- ⚠️ 版本差异提醒：**axum 0.8 起用 `{id}`/`{*path}`（本课程即 0.8）；0.7 及以前是 `:id`/`*path`**

## nest 路由分组

```go
// Gin
api := r.Group("/api/v1")
api.GET("/users", listUsers)      // 实际路径 /api/v1/users
```

```rust
// axum：先把子路由做成独立的 Router，再 nest 挂到前缀下
let api = Router::new()
    .route("/users", get(list_users))    // 这里写相对路径
    .route("/orders", get(list_orders));

let app = Router::new()
    .route("/", get(home))
    .nest("/api/v1", api);               // /api/v1/users、/api/v1/orders
```

比 Gin 更进一步的地方：子 Router 是 **独立的值**，可以放在别的模块/别的文件里组装好再挂上来  
大项目里每个业务域一个 `pub fn routes() -> Router<AppState>`，main 里逐个 nest，结构非常清爽。

## merge 平级合并

```rust
let user_routes = Router::new().route("/users", get(list_users));
let order_routes = Router::new().route("/orders", get(list_orders));
let app = user_routes.merge(order_routes);   // 两张路由表合成一张（无前缀）
```

nest 是「挂到前缀下」，merge 是「原样合并」。Gin 没有直接对应（都用 Group 顶着）。

## fallback 兜底 404

```rust
async fn not_found() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "这里什么都没有")
}

let app = Router::new()
    .route("/", get(home))
    .fallback(not_found);    // 所有没匹配上的请求进这里
```

不设 fallback 时 axum 默认回一个空 body 的 404（≈ `r.NoRoute`）。

## with_state 绑定状态

**不是每次都要。** handler 都不抽 `State<T>` 时，`Router::new().route(...)` 本身就是可 `serve` 的；
只有挂上了需要 `State<AppState>` 的 handler，才必须在末尾 `.with_state(...)`。

```rust
#[derive(Clone)]
struct AppState { /* Arc 字段们 */ }

let app: Router = Router::new()
    .route("/data", get(get_data))   // handler 参数里有 State<AppState>
    .with_state(state);              // 因此这里必须喂一份
```

类型层面发生的事（理解了它，很多报错就看得懂）：

- `Router` 是泛型 `Router<S>`。`Router::new()` 起手是 `Router<()>`——**不欠状态，可以直接 serve**；
- 一旦 `.route` 挂上需要 `State<AppState>` 的 handler，类型变成 `Router<AppState>`（还欠一份状态）；
- `.with_state(state)` 把欠的补上，变回 `Router<()>` —— **只有 `Router<()>` 能交给 `axum::serve`**；
- 所以「该喂却忘了 with_state」不会等到运行时才炸，而是 serve 那行直接编译不过。

nest/merge 的子路由也要状态时：让子 Router 也声明 `Router<AppState>`，在 **最外层** 统一 `.with_state(...)` 一次即可。

----

# Handler 是什么

> axum 里 handler 就是一个满足三个条件的 **async 函数**。

1. 每个参数都是 **提取器**；
2. 返回值实现 **IntoResponse**；
3. 整体是 `Send` 的（见 [《共享状态：Arc / RwLock》](../async/shared-state.md)，通常自动满足）。

不需要注册表、不需要实现接口、不需要 `*gin.Context`——签名即契约：

```rust
// 这些全是合法 handler：
async fn a() -> &'static str { "hi" }                                  // 无参数
async fn b(State(s): State<AppState>) -> Json<Data> { ... }           // 要状态
async fn c(Path(id): Path<u64>, Query(p): Query<Page>) -> ... { ... } // 要多个提取器
```

对照 Gin：所有 handler 都是同一个签名 `func(c *gin.Context)`，要什么自己从 c 里掏；  
axum 反过来， **签名声明需求，框架照单配送**。

----

# 提取器全家桶

> 铁律先放前面（[《读写接口与错误处理》](rest.md) 的规则）：  
> **消费请求体的提取器（Json/Form/String/Bytes）一个 handler 最多一个、必须放参数列表最后**；其余（Path/Query/State/HeaderMap…）随意多个、顺序随意。

## Path 路径参数

```rust
use axum::extract::Path;

// 单参数：/users/{id}
async fn one(Path(id): Path<u64>) -> String {
    format!("id = {id}")                       // ≈ c.Param("id") + strconv.Atoi + 错误处理
}

// 多参数：/users/{uid}/orders/{oid} —— 用元组按顺序接
async fn two(Path((uid, oid)): Path<(u64, u64)>) -> String {
    format!("{uid}/{oid}")
}

// 参数多了用结构体接（字段名 = 路径占位符名）
#[derive(serde::Deserialize)]
struct Ids { uid: u64, oid: u64 }
async fn three(Path(ids): Path<Ids>) -> String {
    format!("{}/{}", ids.uid, ids.oid)
}
```

类型转换失败（`/users/abc` 配 `Path<u64>`）→ 自动回 400，handler 根本不执行。  
Gin 里这是三行 strconv + 手动 400 的活。

## Query 查询串

```rust
use axum::extract::Query;

#[derive(serde::Deserialize)]
struct Pager {
    page: Option<u32>,      // Option = 可不传（?page= 缺失时为 None）
    size: Option<u32>,
    keyword: Option<String>,
}

// GET /list?page=2&keyword=rust
async fn list(Query(p): Query<Pager>) -> String {
    let page = p.page.unwrap_or(1);           // 默认值自己兜
    let size = p.size.unwrap_or(20);
    format!("page={page} size={size} kw={:?}", p.keyword)
}
```

| Gin | axum |
| --- | --- |
| `c.Query("page")`（拿到的是字符串） | 结构体字段自动转好类型 |
| `c.DefaultQuery("page", "1")` | `Option<u32>` + `unwrap_or(1)` |
| 类型错悄悄拿到零值 | 类型错自动 400 |

偷懒兜底：不想定义结构体时 `Query<HashMap<String, String>>` 全收（≈ `c.Request.URL.Query()`）。

## Json 请求体

```rust
use axum::Json;

#[derive(serde::Deserialize)]
struct CreateUser { name: String, age: Option<u8> }

async fn create(Json(body): Json<CreateUser>) -> String {
    format!("创建 {}，年龄 {:?}", body.name, body.age)
}
```

- 必须放参数 **最后**（开头的铁律）；
- body 不是合法 JSON / 缺必填字段 / 类型不符 → 自动回 **422**（Gin 的 ShouldBindJSON 返回 err 需要你自己处理并回 400）；
- 字段名对不上用 serde 改名：`#[serde(rename = "userName")]` ≈ Go tag `json:"userName"`。  
  派生宏、`Option`/`null`、`rename_all`、枚举标签等细讲见 [《JSON 序列化与反序列化》](serde-json.md)。

## Form 表单

```rust
use axum::Form;
async fn login(Form(f): Form<LoginReq>) -> ... { ... }   // Content-Type: x-www-form-urlencoded
```

同样消费 body、同样放最后，用法与 Json 完全平行（≈ `c.PostForm`）。

## State 共享状态

```rust
async fn h(State(state): State<AppState>) -> ... { ... }
```

`State(state)` 这个写法是「参数位置解构」；state 是 `.with_state()` 喂进来的那份的 clone（字段全是 Arc 所以廉价，  
见 [《从零手写 HTTP》](http-from-scratch.md)）。

## HeaderMap 与其他零碎

```rust
use axum::http::HeaderMap;

async fn h(headers: HeaderMap) -> String {
    // ≈ c.GetHeader("User-Agent")
    let ua = headers.get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("-");
    format!("UA: {ua}")
}
```

其余常用提取器速记：

| 想要什么 | 提取器 | Gin 对照 |
| --- | --- | --- |
| 原始 body 字节 | `body: Bytes`（放最后） | `c.GetRawData()` |
| body 当字符串 | `body: String`（放最后） | 同上 + string() |
| 请求方法 | `method: Method` | `c.Request.Method` |
| 完整 URI | `uri: Uri` | `c.Request.URL` |
| 对端地址 | `ConnectInfo<SocketAddr>`（需 serve 时开启） | `c.ClientIP()` |
| 整个原始请求 | `req: Request`（放最后，中间件层常用） | `c.Request` |

----

# 响应全家桶

> handler 的返回值只要实现 `IntoResponse` 就行。常用形态一览（每行都可直接当返回类型/返回值用）。

```rust
async fn text() -> &'static str { "纯文本" }                        // 200 + text/plain
async fn string() -> String { format!("动态文本") }                 // 同上
async fn json() -> Json<Data> { Json(data) }                        // 200 + application/json
async fn html() -> Html<&'static str> { Html("<h1>hi</h1>") }       // 200 + text/html
async fn status_only() -> StatusCode { StatusCode::NO_CONTENT }     // 只有状态码，无 body
async fn with_status() -> (StatusCode, Json<Data>) {                // 自定状态码 + body
    (StatusCode::CREATED, Json(data))                               // ≈ c.JSON(201, data)
}
async fn redirect() -> Redirect { Redirect::to("/login") }          // ≈ c.Redirect(302, "/login")
async fn fallible() -> Result<Json<Data>, (StatusCode, String)> {   // 可失败（读写接口一课）
    Err((StatusCode::NOT_FOUND, "没找到".into()))
}
```

要自定义响应头，在元组里塞 HeaderMap 或用 `[(header::CONTENT_TYPE, "text/csv")]` 数组：

```rust
async fn csv() -> ([(header::HeaderName, &'static str); 1], String) {
    ([(header::CONTENT_TYPE, "text/csv")], "a,b\n1,2".to_string())  // ≈ c.Header(...) + c.String(...)
}
```

**一个高频坑：分支返回不同类型。** Gin 里 `c.JSON`/`c.String` 想调哪个调哪个；Rust 是静态类型，  
函数只能有 **一个** 返回类型，下面的写法编译不过：

```rust
async fn bad(ok: bool) -> ??? {
    if ok { Json(data) } else { StatusCode::BAD_REQUEST }   // ❌ 两个分支类型不同
}
```

两种标准解法：

```rust
// 解法一：每个分支手动 .into_response()，统一成 Response 类型
use axum::response::{IntoResponse, Response};
async fn ok1(ok: bool) -> Response {
    if ok { Json(data).into_response() } else { StatusCode::BAD_REQUEST.into_response() }
}

// 解法二（更常用）：错误分支走 Result 的 Err
async fn ok2(ok: bool) -> Result<Json<Data>, StatusCode> {
    if ok { Ok(Json(data)) } else { Err(StatusCode::BAD_REQUEST) }
}
```

顺带：Gin 的 `gin.H{"msg": "ok"}` 对应 `serde_json::json!` 宏：

```rust
Json(serde_json::json!({ "msg": "ok", "count": 3 }))   // 临时 JSON，不用定义结构体
```

正经接口还是建议定义响应结构体 + derive(Serialize)——编译器帮你把守字段。

----

# 中间件与错误处理

> 这两块已有专课，这里只放地图。

- **中间件**（≈ `r.Use`）：`.layer(middleware::from_fn(f))`，f 的形状是 `async fn(Request, Next) -> Response`，  
  `next.run(req).await` ≈ `c.Next()`—— [《中间件与优雅退出》](middleware-shutdown.md) 从洋葱模型讲到优雅退出；  
  tower-http 的现成件（日志/超时/CORS/压缩）也在那；
- **错误处理**：简单场景 `Result<T, (StatusCode, String)>`（[《读写接口与错误处理》](rest.md)）；  
  工程化的 AppError + From + `?` 整条链（[《接入 Redis》](redis.md)）。

----

# Gin↔axum 速查表

> 收藏用。

| 你在 Gin 里写 | axum 里写 |
| --- | --- |
| `gin.Default()` | `Router::new()` + 自己 layer 日志 |
| `r.GET("/x", h)` | `.route("/x", get(h))` |
| `r.Group("/api")` | `.nest("/api", 子Router)` |
| `r.NoRoute(h)` | `.fallback(h)` |
| `r.Use(mw)` | `.layer(from_fn(mw))` |
| `r.Run(":8080")` | bind + `axum::serve` |
| `c.Param("id")` | `Path(id): Path<u64>` |
| `c.Query("k")` / `DefaultQuery` | `Query<结构体>` + Option/unwrap_or |
| `c.ShouldBindJSON(&x)` | `Json(x): Json<T>`（放最后） |
| `c.PostForm("k")` | `Form<T>` |
| `c.GetHeader("UA")` | `headers: HeaderMap` |
| `c.ClientIP()` | `ConnectInfo<SocketAddr>` |
| `c.JSON(200, x)` | 返回 `Json(x)` |
| `c.JSON(201, x)` | 返回 `(StatusCode::CREATED, Json(x))` |
| `c.String(200, s)` | 返回 `String` / `&'static str` |
| `c.Status(204)` | 返回 `StatusCode::NO_CONTENT` |
| `c.Header(k, v)` | 元组里带 header 数组 |
| `c.Redirect(302, url)` | 返回 `Redirect::to(url)` |
| `gin.H{...}` | `serde_json::json!({...})` |
| `c.Abort()` | 中间件里不调 `next.run` 直接返回 |
| 依赖注入（闭包捕获） | `.with_state` + `State<T>` |

----

# 读懂天书报错

> axum 的 handler 靠 trait 魔法工作，写错时报错常常很吓人。 **九成的报错是同一句**。

```
error[E0277]: the trait bound `fn(...) -> ... {my_handler}: Handler<_, _>` is not satisfied
```

意思是「你这个函数没资格当 handler」。它不告诉你具体哪不对，按下面清单排查（按命中率排序）：

1. **忘写 `async`**：`fn h() -> &'static str` ❌ → `async fn h() ...` ✅；
2. **消费 body 的提取器不在最后**：`(Json(b): Json<T>, State(s): State<S>)` ❌ → State 在前 Json 在后 ✅；
3. **返回类型没实现 IntoResponse**：比如返回了自定义结构体却忘了包 `Json(...)`，或 Result 的 E 不是响应类型；
4. **State 类型对不上**：handler 要 `State<AppState>`，但 `.with_state()` 喂的是别的类型（或压根忘了喂）——这种有时报在 serve 那行；
5. **提取器的泛型参数没实现 Deserialize**：`Query<Pager>` 的 Pager 忘了 `#[derive(Deserialize)]`；
6. **handler 不是 Send**：里面持有了 std 锁 guard / Rc 跨 await（[《共享状态：Arc / RwLock》](../async/shared-state.md) 的老朋友，  
   报错会变成 "future cannot be sent between threads safely"）。

调试技巧：把 `#[axum::debug_handler]` 属性贴在 handler 上（需开 axum 的 `macros` feature），  
报错会从天书变成人话，直接指出上面哪条没满足—— **强烈建议开发期常备**。

----

# 动手实验

1. **把速查表跑一遍**：在 `code/http-rest` 里加三个路由练手——`GET /hello/{name}`（Path）、  
   `GET /search?kw=xx&page=2`（Query+Option 默认值）、`POST /login`（Form）；
2. **体验路由分组**：把 item 相关路由抽成 `fn item_routes() -> Router<AppState>`，  
   用 `.nest("/api", ...)` 挂载，确认老路径 404、新路径 `/api/item/0` 正常；
3. **触发并修复天书报错**：故意把某个 handler 的 `async` 去掉、把 Json 参数挪到第一位、  
   把返回值换成裸结构体——逐一编译，对着上面清单认报错；再贴上 `#[axum::debug_handler]` 看人话版本；
4. **fallback**：给服务加一个返回 JSON 的 404 兜底，`curl` 任意乱路径验证。

----

# 三句话带走

1. **Router 是不可变积木**：route/nest/merge/fallback 层层链式组装；handler 要 `State` 时才用 `with_state` 补上——  
   依赖给没给全是编译期问题，不是运行时惊喜。
2. **提取器按需声明、照单配送**：Path/Query/Json/Form/State/HeaderMap 各管一段请求；  
   消费 body 的只能一个且放最后；解析失败框架自动回 400/422，handler 只写快乐路径。
3. **返回值即响应**：文本/Json/元组带状态码/Redirect/Result 全部开箱即用；分支类型不一致用 `.into_response()` 或 Result 统一；  
   报错读不懂就上 `#[axum::debug_handler]` + 排查清单。
