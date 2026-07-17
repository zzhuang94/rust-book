//! Lesson 08 —— 接入 Redis 的 HTTP 服务。
//!
//! 先准备一个 Redis（任选其一）：
//!   docker run --rm -p 6379:6379 redis
//!   # 或本机已装的 redis-server
//!
//! 运行（可用 REDIS_URL 覆盖默认地址）：
//!   cargo run -p http-redis
//!   REDIS_URL=redis://127.0.0.1:6379/ cargo run -p http-redis
//!
//! 试：
//!   curl http://127.0.0.1:7080/counter          # 每次调用自增
//!   curl -X POST http://127.0.0.1:7080/kv \
//!        -H 'content-type: application/json' \
//!        -d '{"key":"name","value":"tokio","ttl_secs":60}'
//!   curl http://127.0.0.1:7080/kv/name           # tokio
//!   curl http://127.0.0.1:7080/kv/missing        # 404

use axum::routing::{get, post};
use axum::Router;
use labkit::logln;

use http_redis::handler;
use http_redis::state::AppState;

#[tokio::main]
async fn main() {
    let url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());

    // 建立连接。连不上就直接退出并报错（fail fast）。
    let state = AppState::connect(&url)
        .await
        .expect("无法连接 Redis，请确认它已启动、REDIS_URL 正确");
    logln!("已连接 Redis: {url}");

    let app = Router::new()
        .route("/", get(handler::health))
        .route("/counter", get(handler::incr_counter))
        .route("/kv/{key}", get(handler::get_kv))
        .route("/kv", post(handler::set_kv))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:7080").await.unwrap();
    logln!("HTTP 服务已启动：http://127.0.0.1:7080");

    axum::serve(listener, app).await.unwrap();
}
