//! 全局共享状态。
//!
//! 对照 Gin：你通常会把 DB 连接、缓存、配置等做成一个结构体，
//! 用闭包捕获或中间件注入到每个 handler。axum 用 `State<T>` 显式传递，
//! 要求这个 T 是 Clone（这里所有字段都是 Arc，clone 只是加引用计数，很便宜）。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use serde::Serialize;

/// 一份内存数据快照 —— 会被后台任务「整体替换」，而不是逐字段修改。
/// 整体替换 + 读时 clone，是让「读」几乎不被「写」阻塞的关键。
#[derive(Clone, Debug, Serialize)]
pub struct Snapshot {
    pub version: u64,
    pub generated_at_ms: u64,
    pub items: Vec<i64>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Snapshot {
            version: 0,
            generated_at_ms: 0,
            items: Vec::new(),
        }
    }
}

/// 应用全局状态。整体是 Clone 的，因为每个字段都是可廉价克隆的 Arc。
///
/// ★ 为什么必须 #[derive(Clone)]？axum 会为**每个进来的请求** clone 一份 State
///   交给 handler。听起来吓人，其实这里 clone 的只是两个 Arc 句柄（引用计数 +1，
///   第 03 课场景(1)讲过），底层数据全程只有一份 —— 所有请求读写的是同一份。
#[derive(Clone)]
pub struct AppState {
    /// 读多写少：用 RwLock 对应 Go 的 sync.RWMutex。
    pub data: Arc<RwLock<Snapshot>>,
    /// 累计读取次数：无锁原子计数，对应 Go 的 atomic.Int64。
    pub reads: Arc<AtomicU64>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            data: Arc::new(RwLock::new(Snapshot::default())),
            reads: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 读取当前快照（克隆一份返回）。持读锁时间极短，clone 完立即释放。
    pub fn snapshot(&self) -> Snapshot {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.data.read().unwrap().clone()
    }

    /// 累计读取次数。
    pub fn total_reads(&self) -> u64 {
        self.reads.load(Ordering::Relaxed)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
