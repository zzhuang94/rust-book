//! 共享状态 —— 这次带「写」方法。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub version: u64,
    pub generated_at_ms: u64,
    pub items: Vec<i64>,
}

#[derive(Clone)]
pub struct AppState {
    pub data: Arc<RwLock<Snapshot>>,
    pub reads: Arc<AtomicU64>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

impl AppState {
    /// 初始塞几个种子数据，方便直接 GET /item/0 试。
    pub fn new() -> Self {
        let seed = Snapshot {
            version: 1,
            generated_at_ms: now_ms(),
            items: vec![10, 20, 30],
        };
        AppState {
            data: Arc::new(RwLock::new(seed)),
            reads: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 读整份快照。
    pub fn snapshot(&self) -> Snapshot {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.data.read().unwrap().clone()
    }

    /// 按下标读单个元素；越界返回 None（handler 会转成 404）。
    pub fn get_item(&self, idx: usize) -> Option<i64> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.data.read().unwrap().items.get(idx).copied()
    }

    /// 写：追加一个元素，版本号 +1，返回新版本号。
    /// 注意持写锁期间不做任何 .await —— 临界区只有几条内存操作。
    pub fn append_item(&self, value: i64) -> u64 {
        let mut guard = self.data.write().unwrap();
        guard.items.push(value);
        guard.version += 1;
        guard.generated_at_ms = now_ms();
        guard.version
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
