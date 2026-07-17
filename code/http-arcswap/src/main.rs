//! Lesson 06 —— ArcSwap 版 HTTP 服务，接口与第 04 课完全相同。
//!
//! 运行：cargo run -p http-arcswap
//!   curl http://127.0.0.1:7080/data
//!   curl http://127.0.0.1:7080/stats

use std::time::Duration;

use axum::routing::get;
use axum::Router;
use labkit::logln;

use http_arcswap::handler;
use http_arcswap::state::AppState;
use http_arcswap::updater::run_updater;

#[tokio::main]
async fn main() {
    let state = AppState::new();
    tokio::spawn(run_updater(state.clone(), Duration::from_secs(3)));

    let app = Router::new()
        .route("/", get(handler::health))
        .route("/data", get(handler::get_data))
        .route("/stats", get(handler::get_stats))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:7080").await.unwrap();
    logln!("HTTP 服务已启动（ArcSwap 无锁读）：http://127.0.0.1:7080");

    axum::serve(listener, app).await.unwrap();
}
