use std::time::Duration;

use anyhow::Context;
use serde::Deserialize;

/// 这是客户端真正关心的上游协议。
/// 测试假服务也返回这个类型，避免两边各写一份字段名。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Snapshot {
    pub version: u64,
    pub nodes: Vec<String>,
}

/// 生产代码只依赖 Client 和 URL，不知道测试假服务的存在。
/// 这样测试穿过真实 TCP、HTTP 和 JSON 边界，而不是复制实现细节。
pub async fn fetch_snapshot(
    client: &reqwest::Client,
    url: &str,
) -> anyhow::Result<Snapshot> {
    client
        .get(url)
        .send()
        .await
        .with_context(|| format!("请求上游失败：{url}"))?
        .error_for_status()
        .with_context(|| format!("上游返回失败状态：{url}"))?
        .json::<Snapshot>()
        .await
        .with_context(|| format!("上游响应不是合法快照：{url}"))
}

/// 每个测试创建自己的 Client，超时就是该场景的输入之一。
pub fn client_with_timeout(timeout: Duration) -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(timeout)
        .timeout(timeout)
        .build()
        .context("创建 HTTP Client 失败")
}
