//! Lesson 05 —— 读写 REST 接口。
//!
//! 运行：cargo run -p http-rest
//! 试：
//!   curl http://127.0.0.1:7080/data
//!   curl http://127.0.0.1:7080/item/0
//!   curl http://127.0.0.1:7080/item/99                       # 404
//!   curl -X POST http://127.0.0.1:7080/item \
//!        -H 'content-type: application/json' -d '{"value": 42}'

use axum::routing::{get, post};
use axum::Router;
use labkit::logln;

use http_rest::handler;
use http_rest::state::AppState;

#[tokio::main]
async fn main() {
    let state = AppState::new();

    let app = Router::new()
        .route("/", get(handler::health))
        .route("/data", get(handler::get_data))
        .route("/item/{idx}", get(handler::get_item)) // 路径参数（axum 0.8 起用 {} 语法，0.7 及以前是 :idx）
        .route("/item", post(handler::post_item)) // 同一路径不同方法可分别注册
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:7080").await.unwrap();
    logln!("HTTP 服务已启动：http://127.0.0.1:7080");

    axum::serve(listener, app).await.unwrap();
}
