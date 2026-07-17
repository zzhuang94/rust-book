//! 后台更新任务 —— 和第 04 课一样，只是写入从「拿写锁赋值」变成「原子 store」。

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use labkit::logln;

use crate::state::{AppState, Snapshot};

pub async fn run_updater(state: AppState, period: Duration) {
    let mut ticker = tokio::time::interval(period);
    let mut version = 0u64;

    loop {
        ticker.tick().await;
        version += 1;

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let items = (0..version).map(|n| (n * n) as i64).collect();

        // 在锁外（这里根本没有锁）算好新快照，一次原子替换。
        state.store(Snapshot {
            version,
            generated_at_ms: now_ms,
            items,
        });

        logln!("[updater] 已原子替换到 version = {version}");
    }
}
