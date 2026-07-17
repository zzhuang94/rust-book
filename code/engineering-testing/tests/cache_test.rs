//! 异步测试 + 时间控制。
//! #[tokio::test] = 给这个测试配一个专属的 Tokio 运行时（≈ 每个测试自带 main）。

use std::time::Duration;

use engineering_testing::cache::Cache;

#[tokio::test] // 异步测试：函数是 async 的，里面随便 .await
async fn 缓存_写入后能读到() {
    let cache = Cache::new(Duration::from_secs(60));
    cache.set("k", "v").await;
    assert_eq!(cache.get("k").await, Some("v".to_string()));
    assert_eq!(cache.get("不存在").await, None);
}

// ★ 本文件的主角：start_paused = true —— 运行时的时钟从暂停状态开始，
//   tokio::time::advance() 手动快进。"等 61 秒"瞬间完成，测试总耗时毫秒级。
//   对照 Go：要么真 sleep（测试慢），要么给代码注入 Clock 接口（侵入设计）；
//   tokio 因为定时器全归运行时管，测试里直接接管时间，被测代码零改动。
#[tokio::test(start_paused = true)]
async fn 缓存_到期后拿不到() {
    let cache = Cache::new(Duration::from_secs(60));
    cache.set("k", "v").await;

    tokio::time::advance(Duration::from_secs(59)).await; // 快进 59 秒
    assert_eq!(cache.get("k").await, Some("v".to_string()), "还差 1 秒，不该过期");

    tokio::time::advance(Duration::from_secs(2)).await; // 再快进 2 秒（累计 61 秒）
    assert_eq!(cache.get("k").await, None, "超过 TTL，应当过期");
}
