//! 处理函数：演示 JSON 请求体提取、路径参数、以及返回 Result 做错误处理。
//!
//! 提示：`State(state): State<...>`、`Path(idx): Path<...>`、`Json(body): Json<...>`
//! 都是「参数位置的模式解构」——把提取器结构体里的值直接拆出来命名。
//! 这个语法第 04 课 handler.rs 的 get_data 上有详细拆解，忘了就回去看。

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::state::{AppState, Snapshot};

/// GET /data —— 读整份快照。
pub async fn get_data(State(state): State<AppState>) -> Json<Snapshot> {
    Json(state.snapshot())
}

/// GET /item/{idx} —— 读单个元素；越界返回 404。
///
/// 返回 `Result<T, E>`：Ok 分支正常响应，Err 分支被转成错误响应。
/// `(StatusCode, String)` 自带 IntoResponse，是最省事的错误返回方式。
/// 对照 Gin：越界时 `c.JSON(404, gin.H{"error": ...})` 再 `return`。
pub async fn get_item(
    State(state): State<AppState>,
    Path(idx): Path<usize>,
) -> Result<Json<ItemResp>, (StatusCode, String)> {
    match state.get_item(idx) {
        Some(value) => Ok(Json(ItemResp { index: idx, value })),
        None => Err((StatusCode::NOT_FOUND, format!("下标 {idx} 越界"))),
    }
}

/// POST /item —— 追加一个元素。
///
/// 参数里的 `Json(body)` 会把请求体按 JSON 反序列化成 `NewItem`。
/// ⚠️ 顺序要求：消费请求体的提取器（Json）必须放在参数列表**最后**。
/// 对照 Gin：`c.ShouldBindJSON(&body)`。
pub async fn post_item(
    State(state): State<AppState>,
    Json(body): Json<NewItem>,
) -> (StatusCode, Json<AppendResp>) {
    let version = state.append_item(body.value);
    let resp = AppendResp {
        version,
        message: format!("已追加 {}", body.value),
    };
    // 返回元组 (状态码, body)：整体也实现了 IntoResponse。
    (StatusCode::CREATED, Json(resp))
}

/// GET / —— 健康检查。
pub async fn health() -> &'static str {
    "ok"
}

// ---- 请求 / 响应体 ----

#[derive(Deserialize)]
pub struct NewItem {
    pub value: i64,
}

#[derive(Serialize)]
pub struct AppendResp {
    pub version: u64,
    pub message: String,
}

#[derive(Serialize)]
pub struct ItemResp {
    pub index: usize,
    pub value: i64,
}
