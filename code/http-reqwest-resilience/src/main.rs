use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use arc_swap::ArcSwap;
use axum::routing::get;
use axum::{Json, Router};
use labkit::logln;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Snapshot {
    version: u64,
    nodes: Vec<String>,
}

async fn mock_upstream() -> Json<Snapshot> {
    Json(Snapshot {
        version: 2,
        nodes: vec!["node-a".into(), "node-b".into()],
    })
}

async fn fetch(client: &reqwest::Client, url: &str) -> anyhow::Result<Snapshot> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("请求上游失败：{url}"))?
        .error_for_status()
        .with_context(|| format!("上游返回非 2xx：{url}"))?;

    response
        .json::<Snapshot>()
        .await
        .with_context(|| format!("上游 JSON 格式不对：{url}"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 示例自己启动本地上游，不依赖互联网。
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let app = Router::new().route("/snapshot", get(mock_upstream));
    let server = tokio::spawn(async move { axum::serve(listener, app).await });

    // Client 内部复用连接池，应全进程创建一次，而不是每请求 new。
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_millis(300))
        .timeout(Duration::from_secs(1))
        .build()?;

    let cache = Arc::new(ArcSwap::from_pointee(Snapshot {
        version: 1,
        nodes: vec!["fallback".into()],
    }));
    let url = format!("http://{addr}/snapshot");

    match fetch(&client, &url).await {
        Ok(fresh) => {
            cache.store(Arc::new(fresh));
            logln!("刷新成功：{:?}", cache.load_full());
        }
        Err(e) => {
            // soft dependency：刷新失败保留旧快照，不把服务一起打死。
            logln!("刷新失败，继续使用旧快照：{e:#}");
        }
    }

    server.abort();
    Ok(())
}
