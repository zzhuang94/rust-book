//! 测 axum 接口：不 bind 端口、不发真网络包 —— Router 本质是 tower::Service
//! （04 课 §5 讲过），直接喂 Request 拿 Response，全程内存调用。
//! 对照 Go：≈ httptest.NewRecorder + router.ServeHTTP(w, req)，思路完全一样。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt; // .collect() 收响应体，trait 方法要 use 进来（00 课 §6）
use tower::ServiceExt; // .oneshot() 来自它

use engineering_testing::api::app;

#[tokio::test]
async fn 健康检查() {
    let app = app();
    // oneshot：把一个请求喂给 Service，拿到一个响应（用完即弃，最适合测试）
    let resp = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn 加法_正常与参数错误() {
    // 正常：/add?a=1&b=2 → {"sum":3}
    let resp = app()
        .oneshot(Request::builder().uri("/add?a=1&b=2").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 读出 body 字节 → 按 JSON 解析断言（serde_json 是 dev-dependency）
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["sum"], 3);

    // 参数类型错：Query 提取器直接拦下 → 400（04a 课 §4.2），根本不进 handler
    let resp = app()
        .oneshot(Request::builder().uri("/add?a=abc&b=2").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
