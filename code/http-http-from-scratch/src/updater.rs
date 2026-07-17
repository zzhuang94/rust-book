//! 后台更新任务：定期重新生成一份数据，整体替换到共享状态里。
//!
//! 这就是需求里的「定期更新内存数据」。对照 Go：
//!   go func() {
//!       ticker := time.NewTicker(period)
//!       for range ticker.C { /* 重新计算，写入 */ }
//!   }()

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use labkit::logln;

use crate::state::{AppState, Snapshot};

/// 每隔 `period` 生成一份新快照并整体替换旧数据。这个函数不会返回（死循环）。
pub async fn run_updater(state: AppState, period: Duration) {
    let mut ticker = tokio::time::interval(period);
    let mut version = 0u64;

    loop {
        // interval 的第一次 tick 会立即返回，之后每隔 period 触发一次。
        ticker.tick().await;
        version += 1;

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // 模拟「重新计算/从上游拉取」出来的一批新数据。
        let items = (0..version).map(|n| (n * n) as i64).collect();

        let fresh = Snapshot {
            version,
            generated_at_ms: now_ms,
            items,
        };

        // 只有这短短一瞬间持有写锁；读请求几乎不受影响。
        {
            let mut guard = state.data.write().unwrap();
            *guard = fresh;
        }

        logln!("[updater] 内存数据已更新到 version = {version}");
    }
}
