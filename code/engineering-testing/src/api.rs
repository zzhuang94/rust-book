//! 一个极小的 axum 接口：演示"不开端口、纯内存"的接口测试。
//! 要点是把"组装 Router"抽成可复用的函数——生产 main 和测试用同一个 app()。

use axum::extract::Query;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AddParams {
    pub a: i64,
    pub b: i64,
}

#[derive(Serialize)]
pub struct AddResp {
    pub sum: i64,
}

async fn health() -> &'static str {
    "ok"
}

/// GET /add?a=1&b=2 → {"sum":3}
async fn add(Query(p): Query<AddParams>) -> Result<Json<AddResp>, (StatusCode, String)> {
    let sum = p
        .a
        .checked_add(p.b)
        .ok_or((StatusCode::BAD_REQUEST, "溢出了".to_string()))?;
    Ok(Json(AddResp { sum }))
}

/// 组装路由。测试和生产共用这一个入口 —— 可测性的关键一步。
pub fn app() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/add", get(add))
}
