//! Lesson 07 —— 优雅退出 + 中间件日志，并复用第 04 课的库 crate。
//!
//! 两个新知识点：
//!   1. 全局中间件：middleware::from_fn(...)  ≈  Gin 的 r.Use(mw)
//!   2. 优雅退出：serve(...).with_graceful_shutdown(signal)  ≈  http.Server.Shutdown
//!
//! 运行：cargo run -p http-middleware-shutdown
//! 试着访问几个接口，再按 Ctrl-C，观察它「先停止接收新连接、等在途请求结束」后才退出。

use std::time::{Duration, Instant};

use axum::extract::Request;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use labkit::logln;

// 复用 04 的库：state / handler / updater 全都拿来用，不重复实现。
use http_http_from_scratch::handler;
use http_http_from_scratch::state::AppState;
use http_http_from_scratch::updater::run_updater;

#[tokio::main]
async fn main() {
    let state = AppState::new();
    tokio::spawn(run_updater(state.clone(), Duration::from_secs(3)));

    let app = Router::new()
        .route("/", get(handler::health))
        .route("/data", get(handler::get_data))
        .route("/stats", get(handler::get_stats))
        // 全局中间件：每个请求都会经过 log_requests。对照 Gin 的 r.Use(...)。
        .layer(middleware::from_fn(log_requests))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:7080").await.unwrap();
    logln!("HTTP 服务已启动：http://127.0.0.1:7080（按 Ctrl-C 优雅退出）");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    logln!("服务已优雅退出，再见。");
}

/// 请求日志中间件。签名固定为 `(Request, Next) -> Response`。
/// next.run(req) 把请求交给「后面的中间件 / 真正的 handler」，拿到响应后再加工。
async fn log_requests(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    let res = next.run(req).await; // 调用下游

    logln!(
        "[req] {method} {uri} -> {} ({:?})",
        res.status(),
        start.elapsed()
    );
    res
}

/// 等待关闭信号。这里只等 Ctrl-C；生产环境通常还会一起等 SIGTERM。
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("无法安装 Ctrl-C 信号处理器");
    logln!("\n收到 Ctrl-C：停止接收新连接，等待在途请求完成...");
}
