//! Lesson 04 终点站（第 3 步）—— axum 版：组装状态、拉起后台任务、注册路由、监听。
//!
//! 本课是一条"从零到 axum"的四步路（配套文档 docs/04 从 HTTP 协议本身讲起）：
//!   第 0 步 src/bin/step1_single_thread.rs   纯标准库手写 HTTP（单线程阻塞）
//!   第 1 步 src/bin/step2_thread_per_conn.rs 每连接一个线程（≈ Go net/http 模型）
//!   第 2 步 src/bin/step3_tokio_task.rs      线程换成 tokio 任务（异步）
//!   第 3 步 本文件                            hyper+axum 接管 HTTP 协议与路由
//!
//! 运行：cargo run -p http-http-from-scratch
//! 然后另开一个终端：
//!   curl http://127.0.0.1:7080/
//!   curl http://127.0.0.1:7080/data
//!   curl http://127.0.0.1:7080/stats

use std::time::Duration;

use axum::routing::get;
use axum::Router;
use labkit::logln;

use http_http_from_scratch::handler;
use http_http_from_scratch::state::AppState;
use http_http_from_scratch::updater::run_updater;

#[tokio::main]
async fn main() {
    // (1) 构造全局共享状态。
    let state = AppState::new();

    // (2) 拉起后台更新任务：每 3 秒整体刷新一次内存数据。
    //    state.clone() 只是给各字段的 Arc 加引用计数，很便宜。
    tokio::spawn(run_updater(state.clone(), Duration::from_secs(3)));

    // (3) 注册路由。写法和 Gin 的 r.GET("/path", handler) 几乎一一对应。
    let app = Router::new()
        .route("/", get(handler::health))
        .route("/data", get(handler::get_data))
        .route("/stats", get(handler::get_stats))
        .with_state(state);

    // (4) 绑定端口并启动。对照 Gin：r.Run(":7080")。
    let addr = "0.0.0.0:7080";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    logln!("HTTP 服务已启动：http://127.0.0.1:7080");
    logln!("试试：curl http://127.0.0.1:7080/data");

    axum::serve(listener, app).await.unwrap();
}
