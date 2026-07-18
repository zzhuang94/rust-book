//! 信号与进程生命周期：等 Ctrl-C（或 2 秒超时模拟），演示优雅退出的几个步骤
//!
//! 配套文档：docs/concurrency/os-basics.md；呼应 docs/async/service-lifecycle.md
//! 运行：cargo run -p os-signals-lifecycle（先 cd code）
//!
//! "信号"是操作系统通知一个进程"发生了某件事"的机制（Ctrl-C 发的是 SIGINT，
//! Windows 上则是控制台事件，tokio::signal::ctrl_c() 把两者都封装好了，跨平台可用）。
//! 收到退出信号后，一个"优雅"的进程通常要做几件事，而不是立刻猝死：
//!   1. 停止接收新的工作；
//!   2. 通知正在进行的任务"该收尾了"；
//!   3. 给它们一个有限的时间窗口把手头的事做完；
//!   4. 时间到了还没收完，强制结束，打日志说明情况。
//!
//! 为了让这份 demo 不需要人工按 Ctrl-C 也能跑完，这里用 `tokio::select!` 同时等
//! "真正的 Ctrl-C" 和 "2 秒超时"，谁先到就走谁——超时相当于模拟"收到了退出信号"。

use std::time::Duration;

use anyhow::Context;
use labkit::logln;
use tokio::time::sleep;

/// 模拟一个正在运行的后台任务：定期打心跳，收到退出信号后還要跑完"手头这一件事"才收尾。
async fn worker(id: u32, mut shutdown: tokio::sync::watch::Receiver<bool>) {
    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                logln!("  worker#{id} 收到停机通知，开始收尾（模拟耗时 300ms 的清理工作）");
                sleep(Duration::from_millis(300)).await;
                logln!("  worker#{id} 收尾完成，退出");
                return;
            }
            _ = sleep(Duration::from_millis(400)) => {
                logln!("  worker#{id} 心跳：仍在正常工作");
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logln!("--- 优雅退出演示：等待 Ctrl-C，或 2 秒后自动模拟「收到退出信号」 ---");
    logln!("（想真实体验的话，2 秒内按下 Ctrl-C 也一样会触发下面的收尾流程）");

    // watch 通道：广播"要不要关闭"这个状态，所有 worker 都能收到同一条通知。
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    let mut workers = tokio::task::JoinSet::new();
    for id in 1..=3 {
        workers.spawn(worker(id, shutdown_rx.clone()));
    }

    // 步骤 1：等待"退出信号"——真正的 Ctrl-C，或者 2 秒超时模拟。
    tokio::select! {
        signal = tokio::signal::ctrl_c() => {
            signal.context("安装 Ctrl-C 信号处理器失败")?;
            logln!("[步骤1] 收到真实的 Ctrl-C 信号");
        }
        _ = sleep(Duration::from_secs(2)) => {
            logln!("[步骤1] 2 秒到了，模拟「收到退出信号」（没有真的按 Ctrl-C 也没关系）");
        }
    }

    // 步骤 2：停止接收新工作（这里没有"新工作入口"，用日志代表这一步）。
    logln!("[步骤2] 停止接收新的工作请求");

    // 步骤 3：通知所有 worker 该收尾了。
    logln!("[步骤3] 通过 watch 通道通知所有 worker 开始收尾");
    shutdown_tx.send(true).ok();

    // 步骤 4：给它们一个有限的时间窗口。
    logln!("[步骤4] 给 worker 们最多 2 秒时间收尾");
    let drain = async {
        while let Some(res) = workers.join_next().await {
            match res {
                Ok(()) => logln!("  一个 worker 已确认退出"),
                Err(e) => logln!("  一个 worker 异常退出：{e}"),
            }
        }
    };
    if tokio::time::timeout(Duration::from_secs(2), drain).await.is_err() {
        logln!("[步骤4] 收尾超时，强制终止剩余 worker（abort_all）");
        workers.abort_all();
    } else {
        logln!("[步骤4] 所有 worker 都在时间窗口内正常收尾");
    }

    logln!("[步骤5] 优雅退出完成，进程正常结束");
    Ok(())
}
