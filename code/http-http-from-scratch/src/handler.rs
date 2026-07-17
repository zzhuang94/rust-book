//! HTTP 处理函数。每个 async fn 就是一个「路由处理器」，对应 Gin 的 HandlerFunc。
//!
//! axum 的魔法：函数参数（如 `State<AppState>`）会被自动从请求里「提取」出来，
//! 返回值（如 `Json<T>`、`&str`）会被自动转成 HTTP 响应。
//! 对照 Gin：Gin 是把 `*gin.Context` 传进来，你从 c 上手动取参数、手动写响应。

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::state::{AppState, Snapshot};

/// GET /  —— 健康检查。对照 Gin：c.String(200, "ok")。
/// 返回 &'static str（字符串字面量），axum 自动把它变成 200 + text/plain 响应。
pub async fn health() -> &'static str {
    "ok"
}

/// GET /data —— 返回当前内存快照。对照 Gin：c.JSON(200, data)。
///
/// ★ 语法拆解（小白必看）：参数 `State(state): State<AppState>` 是「参数位置的模式解构」，
///   和 `let State(state) = ...;` 是同一个语法 —— State 是个单字段元组结构体
///   （struct State<T>(T)），这样写直接把里面的 AppState 拆出来命名为 state。
///   等价的啰嗦写法：`s: State<AppState>` 然后用 `s.0`。
///
/// ★ 返回 Json<Snapshot>：axum 会自动把 Snapshot 序列化成 JSON、
///   设置 Content-Type: application/json。前提是 Snapshot 实现了 serde 的
///   Serialize（见 state.rs 里的 #[derive(Serialize)]，≈ Go 的 json tag）。
pub async fn get_data(State(state): State<AppState>) -> Json<Snapshot> {
    Json(state.snapshot())
}

/// /stats 的响应体。
#[derive(Serialize)]
pub struct Stats {
    pub version: u64,
    pub item_count: usize,
    pub total_reads: u64,
}

/// GET /stats —— 返回一些运行时统计信息。
pub async fn get_stats(State(state): State<AppState>) -> Json<Stats> {
    let snap = state.snapshot();
    Json(Stats {
        version: snap.version,
        item_count: snap.items.len(),
        total_reads: state.total_reads(),
    })
}
