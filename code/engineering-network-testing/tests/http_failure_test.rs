use std::time::Duration;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use engineering_network_testing::{client_with_timeout, fetch_snapshot};
use serde_json::json;

/// 持有后台服务任务。测试结束时 Drop 自动 abort，避免残留孤儿任务。
struct TestServer {
    url: String,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// listener 在 spawn 前已经 bind 完成，所以返回后服务一定占住了端口，
/// 不需要 sleep 猜“它启动好没有”。端口 0 让并行测试互不抢端口。
async fn serve(app: Router) -> TestServer {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("绑定测试端口失败");
    let addr = listener.local_addr().expect("读取测试地址失败");
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("测试 HTTP 服务异常退出");
    });
    TestServer {
        url: format!("http://{addr}/snapshot"),
        task,
    }
}

#[tokio::test]
async fn 真实_http_成功响应能被解析() {
    let app = Router::new().route(
        "/snapshot",
        get(|| async { Json(json!({"version": 7, "nodes": ["node-a"]})) }),
    );
    let server = serve(app).await;
    let client = client_with_timeout(Duration::from_secs(1)).unwrap();

    let snapshot = fetch_snapshot(&client, &server.url).await.unwrap();

    assert_eq!(snapshot.version, 7);
    assert_eq!(snapshot.nodes, ["node-a"]);
}

#[tokio::test]
async fn 非二百状态不会被当成成功() {
    let app = Router::new().route(
        "/snapshot",
        get(|| async { (StatusCode::SERVICE_UNAVAILABLE, "稍后再试") }),
    );
    let server = serve(app).await;
    let client = client_with_timeout(Duration::from_secs(1)).unwrap();

    let error = fetch_snapshot(&client, &server.url).await.unwrap_err();

    assert!(format!("{error:#}").contains("失败状态"));
}

#[tokio::test]
async fn 坏_json_会保留解析现场() {
    let app = Router::new().route(
        "/snapshot",
        get(|| async { ([("content-type", "application/json")], "{broken") }),
    );
    let server = serve(app).await;
    let client = client_with_timeout(Duration::from_secs(1)).unwrap();

    let error = fetch_snapshot(&client, &server.url).await.unwrap_err();

    assert!(format!("{error:#}").contains("不是合法快照"));
}

#[tokio::test]
async fn 慢响应会触发客户端超时() {
    let app = Router::new().route(
        "/snapshot",
        get(|| async {
            tokio::time::sleep(Duration::from_millis(500)).await;
            Json(json!({"version": 8, "nodes": []})).into_response()
        }),
    );
    let server = serve(app).await;
    let client = client_with_timeout(Duration::from_millis(50)).unwrap();

    let error = fetch_snapshot(&client, &server.url).await.unwrap_err();

    assert!(format!("{error:#}").contains("请求上游失败"));
}

#[tokio::test]
async fn 对端接收后断开会成为传输错误() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        drop(socket); // 不返回任何 HTTP 字节，直接模拟连接中断。
    });
    let client = client_with_timeout(Duration::from_secs(1)).unwrap();

    let result = fetch_snapshot(&client, &format!("http://{addr}/snapshot")).await;

    assert!(result.is_err());
    task.await.unwrap();
}
