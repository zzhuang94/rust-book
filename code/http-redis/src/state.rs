//! 共享状态 —— 这次装的是一个 Redis 连接。
//!
//! `ConnectionManager` 的关键特性：
//!   - 内部是「多路复用」的单条连接：多个并发请求共用它，无需连接池即可并发。
//!   - 可 Clone（clone 只是共享同一底层连接），所以能直接当 axum 的 State。
//!   - 自带断线重连。
//!
//! 对照 Go：go-redis 的 `*redis.Client` 本身是并发安全的、内置连接池，
//! 你也是全局建一个、到处共享。心智一致。

use redis::aio::ConnectionManager;

#[derive(Clone)]
pub struct AppState {
    pub redis: ConnectionManager,
}

impl AppState {
    /// 连接 Redis。url 形如 "redis://127.0.0.1:6379/"。
    /// 对照 Go：redis.NewClient(&redis.Options{Addr: "..."}).
    pub async fn connect(url: &str) -> redis::RedisResult<Self> {
        let client = redis::Client::open(url)?;
        let redis = ConnectionManager::new(client).await?;
        Ok(AppState { redis })
    }
}
