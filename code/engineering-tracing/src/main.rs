//! Lesson 12 —— tracing 结构化日志：告别 println，接上生产级可观测性。
//!
//! 三个角色（对照 Go）：
//!   tracing              ≈ slog 的门面：info!/warn! 宏 + span 概念
//!   tracing-subscriber   ≈ slog 的 handler：决定日志去哪、什么格式、过滤哪些
//!   tower-http TraceLayer≈ gin.Logger() 中间件：每个 HTTP 请求自动一个 span
//!
//! 运行：cargo run -p engineering-tracing
//! 换过滤级别：RUST_LOG=debug cargo run -p engineering-tracing
//! 只看某模块：RUST_LOG=engineering_tracing=debug,tower_http=warn cargo run -p engineering-tracing
//! 然后访问：curl "http://127.0.0.1:7080/order?user_id=42&amount=99"

use std::time::Duration;

use axum::extract::Query;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::trace::TraceLayer;
use tracing::{debug, info, instrument, warn};

#[derive(Deserialize)]
struct OrderParams {
    user_id: u64,
    amount: i64,
}

#[derive(Serialize)]
struct OrderResp {
    order_id: u64,
}

#[tokio::main]
async fn main() {
    // (1) 初始化订阅端：决定"日志输出到哪、什么格式、放行哪些级别"。
    //     EnvFilter 读环境变量 RUST_LOG（≈ Go 各日志库的 LOG_LEVEL 约定，但支持按模块细分）。
    //     不设 RUST_LOG 时用这里的默认值：本 crate debug，其余 info。
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tracing_lab=debug".into()),
        )
        .with_target(true) // 打印日志来源模块
        .init();

    // (2) 事件（event）：某一时刻发生了一件事 —— 带结构化字段，不是拼字符串！
    //     字段写法 key = value；?value 用 Debug 格式；%value 用 Display 格式。
    info!(version = "1.0", "服务启动中");

    // 后台任务的日志会自动带上模块路径，便于 RUST_LOG 按模块过滤
    tokio::spawn(async {
        let mut ticker = tokio::time::interval(Duration::from_secs(5));
        loop {
            ticker.tick().await;
            debug!(pending_jobs = 3, "后台巡检"); // debug 级：默认配置下能看到（本 crate 是 debug）
        }
    });

    let app = Router::new()
        .route("/order", get(create_order))
        // (3) TraceLayer：每个请求自动创建一个 span，请求开始/结束/耗时/状态码全自动。
        //     ≈ gin.Logger()，但产物是结构化的、可接入链路追踪。
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:7080").await.unwrap();
    info!("HTTP 服务已启动：http://127.0.0.1:7080");
    axum::serve(listener, app).await.unwrap();
}

/// (4) #[instrument]：给整个函数包一个 span（带时间范围的上下文）。
///     函数参数自动成为 span 的字段；skip 掉不想记录的参数。
///     函数内所有日志、以及它调用的函数里的日志，都会自动带上这个 span 的字段 ——
///     这就是 Rust 版的"请求上下文"：Go 里你靠 ctx + 手动把 user_id 传给每层日志，
///     这里 span 一包，下游日志自动携带。
#[instrument(skip(params), fields(user_id = params.user_id))]
async fn create_order(Query(params): Query<OrderParams>) -> Json<OrderResp> {
    info!(amount = params.amount, "收到下单请求");

    if params.amount > 1000 {
        warn!(amount = params.amount, "大额订单，触发风控检查");
    }

    let order_id = save_order(params.amount).await;
    info!(order_id, "下单完成");
    Json(OrderResp { order_id })
}

/// 下游函数：自己也有 span；它打的日志会同时带着上游 create_order 的 user_id 字段
/// （span 是嵌套的——这就是"链路"二字的最小形态）。
#[instrument]
async fn save_order(amount: i64) -> u64 {
    debug!("开始写库");
    tokio::time::sleep(Duration::from_millis(50)).await; // 模拟 IO
    debug!("写库完成");
    7001
}
