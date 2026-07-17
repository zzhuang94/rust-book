//! 处理函数 —— 读路径完全无锁。

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::state::{AppState, Snapshot};

pub async fn health() -> &'static str {
    "ok"
}

/// GET /data —— 无锁读，直接把 Arc<Snapshot> 交给 axum 序列化。
/// 因为工作区给 serde 开了 `rc` 特性，Arc<T> 可以直接被序列化，连结构体都不用克隆。
pub async fn get_data(State(state): State<AppState>) -> Json<Arc<Snapshot>> {
    Json(state.load())
}

#[derive(Serialize)]
pub struct Stats {
    pub version: u64,
    pub item_count: usize,
    pub total_reads: u64,
}

pub async fn get_stats(State(state): State<AppState>) -> Json<Stats> {
    let snap = state.load();
    Json(Stats {
        version: snap.version,
        item_count: snap.items.len(),
        total_reads: state.total_reads(),
    })
}
