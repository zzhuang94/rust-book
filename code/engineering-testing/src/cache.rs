//! 异步 TTL 缓存：演示"怎么测异步代码 + 怎么测和时间有关的逻辑"。
//!
//! 关键选型：时间用 tokio::time::Instant（而不是 std 的）——
//! 它服从测试里的"时间暂停/快进"（tokio::time::pause / advance），
//! 于是"过期 60 秒后……"的测试**瞬间**跑完，不用真等。

use std::collections::HashMap;
use std::time::Duration;

use tokio::sync::RwLock;
use tokio::time::Instant;

pub struct Cache {
    ttl: Duration,
    inner: RwLock<HashMap<String, (String, Instant)>>, // 值 + 写入时刻
}

impl Cache {
    pub fn new(ttl: Duration) -> Self {
        Cache { ttl, inner: RwLock::new(HashMap::new()) }
    }

    pub async fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        let mut m = self.inner.write().await;
        m.insert(key.into(), (value.into(), Instant::now()));
    }

    /// 取值；不存在或已过期返回 None。
    pub async fn get(&self, key: &str) -> Option<String> {
        let m = self.inner.read().await;
        let (value, written_at) = m.get(key)?;
        if written_at.elapsed() > self.ttl {
            return None; // 过期（惰性判定，演示用，不做真删除）
        }
        Some(value.clone())
    }
}
