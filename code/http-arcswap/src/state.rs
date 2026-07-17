//! 用 ArcSwap 承载共享状态。
//!
//! 核心思想（RCU：Read-Copy-Update）：
//!   - 数据整份放在一个「可原子交换的指针」里。
//!   - 读：原子地取出当前 Arc，无锁、O(1)、绝不阻塞。
//!   - 写：在旁边算好新版本，一次原子替换指针；老版本等最后一个读者用完自动回收。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use arc_swap::ArcSwap;
use serde::Serialize;

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

/// 真正的共享数据放这里；AppState 只是包一层 Arc 好让它可 Clone。
struct Inner {
    data: ArcSwap<Snapshot>,
    reads: AtomicU64,
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            inner: Arc::new(Inner {
                data: ArcSwap::new(Arc::new(Snapshot::default())),
                reads: AtomicU64::new(0),
            }),
        }
    }

    /// 无锁读取：返回一个指向当前快照的 Arc，代价只是引用计数 +1。
    /// 注意返回的是 `Arc<Snapshot>` 而不是克隆出的 `Snapshot` —— 没有拷贝数据。
    pub fn load(&self) -> Arc<Snapshot> {
        self.inner.reads.fetch_add(1, Ordering::Relaxed);
        self.inner.data.load_full()
    }

    /// 写：原子替换整份快照。老版本会在最后一个持有者释放后被回收。
    pub fn store(&self, snap: Snapshot) {
        self.inner.data.store(Arc::new(snap));
    }

    pub fn total_reads(&self) -> u64 {
        self.inner.reads.load(Ordering::Relaxed)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
