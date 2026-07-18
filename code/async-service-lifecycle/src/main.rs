use std::time::Duration;

use anyhow::Context;
use labkit::logln;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

async fn heartbeat(cancel: CancellationToken) -> anyhow::Result<()> {
    let mut ticker = tokio::time::interval(Duration::from_millis(300));
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                logln!("heartbeat 收到取消，做完收尾后退出");
                return Ok(());
            }
            _ = ticker.tick() => logln!("heartbeat: still alive"),
        }
    }
}

async fn updater(cancel: CancellationToken) -> anyhow::Result<()> {
    tokio::select! {
        _ = cancel.cancelled() => Ok(()),
        _ = tokio::time::sleep(Duration::from_secs(10)) => {
            anyhow::bail!("上游配置连续刷新失败")
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cancel = CancellationToken::new();
    let mut tasks = JoinSet::new();

    tasks.spawn(heartbeat(cancel.child_token()));
    tasks.spawn(updater(cancel.child_token()));

    tokio::select! {
        signal = tokio::signal::ctrl_c() => {
            signal.context("安装 Ctrl-C 信号处理器失败")?;
            logln!("收到 Ctrl-C，开始停机");
        }
        result = tasks.join_next() => {
            match result {
                Some(Ok(Ok(()))) => logln!("关键任务提前正常结束"),
                Some(Ok(Err(e))) => logln!("关键任务失败：{e:#}"),
                Some(Err(e)) => logln!("关键任务 panic 或被取消：{e}"),
                None => logln!("没有任务可监管"),
            }
        }
    }

    cancel.cancel();

    let drain = async {
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(())) => logln!("一个任务已干净退出"),
                Ok(Err(e)) => logln!("一个任务带错误退出：{e:#}"),
                Err(e) => logln!("一个任务未正常完成：{e}"),
            }
        }
    };

    if tokio::time::timeout(Duration::from_secs(2), drain)
        .await
        .is_err()
    {
        logln!("收尾超时，强制终止剩余任务");
        tasks.abort_all();
    }

    logln!("所有后台任务处理完毕，主进程退出");
    Ok(())
}
