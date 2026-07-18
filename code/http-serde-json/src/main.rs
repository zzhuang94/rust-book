//! HTTP 服务 · JSON 序列化与反序列化 —— 可运行示例
//!
//! 配套文档：docs/http/serde-json.md
//! 运行：cargo run -p http-serde-json（先 cd code）
//!
//! 对照 Go：encoding/json 的 Marshal / Unmarshal / json 标签。
//! 本课不启 HTTP 服务；axum 的 Json<T> 只是在边界调用同一套 serde。

use labkit::logln;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

fn main() {
    demo_roundtrip();
    demo_value_and_json_macro();
    demo_rename_and_rename_all();
    demo_option_null_missing();
    demo_default_and_skip();
    demo_flatten();
    demo_enum_tagging();
    demo_error_messages();
    demo_axum_mental_model();
}

// ======================== 基础往返 ========================

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
struct User {
    id: u64,
    name: String,
    active: bool,
}

fn demo_roundtrip() {
    logln!("--- 结构体 ↔ JSON 往返 ---");

    let u = User {
        id: 1,
        name: "张三".into(),
        active: true,
    };

    // 对照 Go：json.Marshal
    let s = serde_json::to_string(&u).unwrap();
    logln!("  to_string = {s}");

    let pretty = serde_json::to_string_pretty(&u).unwrap();
    logln!("  pretty:\n{pretty}");

    // 对照 Go：json.Unmarshal
    let back: User = serde_json::from_str(&s).unwrap();
    logln!("  from_str 回来相等? {}", back == u);

    // 字节版：对 HTTP body / 文件更自然
    let bytes = serde_json::to_vec(&u).unwrap();
    let from_bytes: User = serde_json::from_slice(&bytes).unwrap();
    logln!("  from_slice ok, name={}", from_bytes.name);
}

// ======================== Value ========================

fn demo_value_and_json_macro() {
    logln!("--- Value 与 json! ---");

    // 对照 Go：map[string]any / json.RawMessage 的部分场景
    let v: Value = json!({
        "id": 7,
        "tags": ["rust", "go"],
        "meta": { "ok": true }
    });
    logln!("  json! = {v}");
    logln!("  指针取值 v[\"id\"] = {}", v["id"]);
    logln!("  安全取值 = {:?}", v.get("tags").and_then(|t| t.get(0)));

    // 动态 JSON → 强类型（部分字段）
    #[derive(Deserialize, Debug)]
    struct OnlyId {
        id: u64,
    }
    let only: OnlyId = serde_json::from_value(v).unwrap();
    logln!("  只取 id → {only:?}, id={}", only.id);
}

// ======================== 字段命名 ========================

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")] // 对照 Go：结构体字段导出 + json:"userId"
struct ApiUser {
    user_id: u64,
    display_name: String,
    #[serde(rename = "isAdmin")] // 单个字段覆盖 rename_all
    admin: bool,
}

fn demo_rename_and_rename_all() {
    logln!("--- rename / rename_all ---");

    let u = ApiUser {
        user_id: 9,
        display_name: "admin".into(),
        admin: true,
    };
    let s = serde_json::to_string(&u).unwrap();
    logln!("  序列化（camelCase）= {s}");

    let raw = r#"{"userId":9,"displayName":"admin","isAdmin":true}"#;
    let back: ApiUser = serde_json::from_str(raw).unwrap();
    logln!("  反序列化 display_name={}", back.display_name);
}

// ======================== Option / null / 缺字段 ========================

#[derive(Debug, Serialize, Deserialize)]
struct Patch {
    /// 缺字段或 null → None；有值 → Some
    title: Option<String>,
    /// 用下面两个属性区分「缺字段」和「显式 null」时见文档；
    /// 日常 PATCH 用 Option 往往够用。
    views: Option<u64>,
}

fn demo_option_null_missing() {
    logln!("--- Option：缺字段 vs null ---");

    let a: Patch = serde_json::from_str(r#"{"title":"hi","views":3}"#).unwrap();
    let b: Patch = serde_json::from_str(r#"{"title":null}"#).unwrap(); // views 缺
    let c: Patch = serde_json::from_str(r#"{}"#).unwrap();

    logln!("  全有 = {a:?}");
    logln!("  title=null, views 缺 = {b:?}");
    logln!("  全缺 = {c:?}");

    // 序列化：None 默认输出 null（字段还在）
    let p = Patch {
        title: None,
        views: Some(1),
    };
    logln!("  None 默认序列化 = {}", serde_json::to_string(&p).unwrap());
}

// ======================== default / skip ========================

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    #[serde(default)] // 缺字段 → Default::default()，这里是 0
    retries: u32,
    #[serde(default = "default_timeout")]
    timeout_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    #[serde(skip)] // 序列化、反序列化都忽略（常用于内部缓存字段）
    #[serde(default)]
    cache_hit: bool,
}

fn default_timeout() -> u64 {
    3000
}

fn demo_default_and_skip() {
    logln!("--- default / skip ---");

    let c: Config = serde_json::from_str(r#"{"note":"x"}"#).unwrap();
    logln!("  缺省填充 = {c:?}");

    let out = Config {
        retries: 1,
        timeout_ms: 100,
        note: None,
        cache_hit: true, // skip：不会出现在 JSON 里
    };
    logln!("  skip_serializing_if None → {}", serde_json::to_string(&out).unwrap());
}

// ======================== flatten ========================

#[derive(Debug, Serialize, Deserialize)]
struct Page {
    limit: u32,
    offset: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Search {
    q: String,
    #[serde(flatten)]
    page: Page, // 展开到同一层，而不是嵌套 "page":{...}
}

fn demo_flatten() {
    logln!("--- flatten ---");

    let raw = r#"{"q":"rust","limit":10,"offset":0}"#;
    let s: Search = serde_json::from_str(raw).unwrap();
    logln!("  flatten 反序列化 = {s:?}");
    logln!("  再序列化 = {}", serde_json::to_string(&s).unwrap());
}

// ======================== 枚举 tagging ========================

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")] //  internally adjacent：{"type":"Login","data":{...}}
enum Event {
    Login { user: String },
    Logout { user: String },
    Ping,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Simple {
    Started,
    Failed { reason: String },
}

fn demo_enum_tagging() {
    logln!("--- 枚举序列化 ---");

    // 默认外部标签：{"Failed":{"reason":"timeout"}}
    let e = Simple::Failed {
        reason: "timeout".into(),
    };
    logln!("  默认外部标签 = {}", serde_json::to_string(&e).unwrap());

    let login = Event::Login {
        user: "alice".into(),
    };
    let s = serde_json::to_string(&login).unwrap();
    logln!("  adjacently tagged = {s}");
    let back: Event = serde_json::from_str(&s).unwrap();
    logln!("  回来 = {back:?}");
}

// ======================== 错误信息 ========================

fn demo_error_messages() {
    logln!("--- 反序列化错误（可读）---");

    let bad = r#"{"id":"not-a-number","name":"x","active":true}"#;
    match serde_json::from_str::<User>(bad) {
        Ok(_) => logln!("  意外成功"),
        Err(e) => logln!("  err = {e}"),
    }

    let missing = r#"{"id":1}"#; // 缺 name/active
    match serde_json::from_str::<User>(missing) {
        Ok(_) => logln!("  意外成功"),
        Err(e) => logln!("  缺字段 err = {e}"),
    }
}

// ======================== 和 axum 的关系 ========================

fn demo_axum_mental_model() {
    logln!("--- 和 axum Json<T> 的关系（心智模型）---");
    logln!("  请求：body 字节 → serde_json::from_slice → T（Deserialize）");
    logln!("  响应：T（Serialize）→ serde_json::to_vec → body 字节");
    logln!("  你在 handler 里写 Json(user)，框架替你调用上面两步");
    logln!("  下一课 rest.md 会把它接到真正的 HTTP 路由上");
}
