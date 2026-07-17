//! Lesson 10 —— 通知与热更新：watch / broadcast / Notify / OnceCell。
//!
//! 对照 Go 的心智映射：
//!   (1) watch      ≈ atomic 存最新值 + 通知（Go 里要手搓）—— 配置热更新神器
//!   (2) broadcast  ≈ 给每个订阅者各开一个 chan 再手动 fan-out
//!   (3) Notify     ≈ sync.Cond / 空 struct{} 的信号 chan
//!   (4) OnceCell   ≈ sync.Once（但初始化过程可以是 async 的）
//!
//! 运行：cargo run -p async-notify-watch

use std::sync::Arc;
use std::time::Duration;

use labkit::logln;
use tokio::sync::{broadcast, watch, Notify, OnceCell};
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    // =====================================================================
    // (1) watch：只保留「最新值」的通道 —— 配置热更新
    // =====================================================================
    // 和 mpsc 的本质区别：mpsc 是队列（每条消息都要被消费）；
    // watch 是"一格储物柜"（新值直接覆盖旧值，读者永远只看最新）。
    // 这正是配置热更新想要的语义：错过的中间版本根本不重要。
    //
    // 对照 Go：没有现成原语，通常 atomic.Pointer 存配置 + 另开 chan 通知，
    // 两件事要自己拼。watch 一个类型两件全包。
    //
    // ★ 它也是本项目「定期更新 + 高并发读」的第三种解法：
    //   RwLock（04 课）/ ArcSwap（06 课）/ watch（本课）——
    //   watch 独有的能力是"值变了可以异步等到通知"。
    logln!("(1) watch：配置热更新 ------------------------------------------------");
    let (cfg_tx, cfg_rx) = watch::channel(0u64); // 初始配置版本 0

    // 两个读者：故意读得慢（900ms 一轮），演示"跳过中间版本、只看最新"
    let mut readers = Vec::new();
    for id in 1..=2 {
        let mut rx = cfg_rx.clone(); // watch 的接收端可以随便 clone（多读者）
        readers.push(tokio::spawn(async move {
            // changed()：异步等"值发生变化"；发送端 drop 后返回 Err → 退出
            while rx.changed().await.is_ok() {
                // borrow()：读当前最新值（借用，读完尽快放，别跨 await 持有）
                let v = *rx.borrow();
                logln!("    读者{id} 看到配置版本 = {v}");
                sleep(Duration::from_millis(900)).await; // 慢读者
            }
            logln!("    读者{id}：写端已关闭，退出");
        }));
    }

    // 写者：每 400ms 发布一个新版本，共 5 版
    for v in 1..=5u64 {
        sleep(Duration::from_millis(400)).await;
        logln!("发布配置版本 {v}");
        cfg_tx.send(v).unwrap(); // 覆盖储物柜里的旧值
    }
    drop(cfg_tx); // 写端消失 → 读者的 changed() 返回 Err，退出循环
    for r in readers {
        r.await.unwrap();
    }
    logln!("★ 读者每轮 900ms、写者每 400ms 发一版：读者看到的版本跳着走");
    logln!("  （比如 1→3→5），中间版本被覆盖 —— 这正是配置热更新要的语义\n");

    // =====================================================================
    // (2) broadcast：一条消息，每个订阅者都收到
    // =====================================================================
    // 和 mpsc 的区别：mpsc 一条消息只被**一个**消费者拿到（抢活干）；
    // broadcast 每个订阅者都收到**每一条**（广播事件）。
    // 对照 Go：得自己维护 []chan 逐个发，还要处理慢订阅者 —— broadcast 全包。
    // 注意：容量满时最慢的订阅者会丢最旧的消息并收到 Lagged 错误（防慢者拖垮全局）。
    logln!("(2) broadcast：事件扇出 ----------------------------------------------");
    let (event_tx, _) = broadcast::channel::<String>(8);
    let mut subs = Vec::new();
    for id in 1..=2 {
        let mut rx = event_tx.subscribe(); // 每个订阅者一个独立接收端
        subs.push(tokio::spawn(async move {
            // recv 返回 Err(Closed) = 发送端全部 drop → 退出
            while let Ok(msg) = rx.recv().await {
                logln!("    订阅者{id} 收到事件: {msg}");
            }
            logln!("    订阅者{id}：事件源已关闭，退出");
        }));
    }
    for i in 1..=3 {
        event_tx.send(format!("事件-{i}")).unwrap();
        sleep(Duration::from_millis(200)).await;
    }
    drop(event_tx);
    for s in subs {
        s.await.unwrap();
    }
    logln!("★ 每条事件两个订阅者都收到了（对比 mpsc：一条只会被一个人抢走）\n");

    // =====================================================================
    // (3) Notify：纯事件通知（不带数据）
    // =====================================================================
    // "有事发生了，醒一醒"——不需要传值时，用 Notify 比开个 channel 更轻。
    // 对照 Go：sync.Cond，或者惯用的 make(chan struct{}) + close/发送。
    logln!("(3) Notify：纯信号 ----------------------------------------------------");
    let notify = Arc::new(Notify::new());
    let waiter = {
        let notify = Arc::clone(&notify);
        tokio::spawn(async move {
            logln!("    等待者：睡在 notified() 上，等人叫我");
            notify.notified().await;
            logln!("    等待者：被唤醒，开工！");
        })
    };
    sleep(Duration::from_millis(800)).await;
    logln!("main：叫醒等待者（notify_one）");
    notify.notify_one(); // 唤醒一个等待者；notify_waiters() 则唤醒全部
    waiter.await.unwrap();
    logln!("");

    // =====================================================================
    // (4) OnceCell：异步懒初始化（只初始化一次的全局资源）
    // =====================================================================
    // 需求：第一次用到时才建立"连接"（耗时 500ms），之后所有人复用；
    // 多个任务同时抢着初始化时，只有一个真正执行，其余等结果。
    // 对照 Go：sync.Once —— 但 Once 的 f 不能 await；
    // tokio 的 OnceCell 允许初始化过程本身是 async 的。
    logln!("(4) OnceCell：异步懒初始化 --------------------------------------------");
    static CONN: OnceCell<String> = OnceCell::const_new();

    async fn get_conn() -> &'static String {
        CONN.get_or_init(|| async {
            logln!("    正在建立连接（耗时 500ms，这行只会出现一次！）");
            sleep(Duration::from_millis(500)).await;
            "连接#001".to_string()
        })
        .await
    }

    // 3 个任务同时要连接：只有 1 次初始化，其余 2 个等着复用
    let mut users = Vec::new();
    for id in 1..=3 {
        users.push(tokio::spawn(async move {
            let conn = get_conn().await;
            logln!("    任务{id} 拿到 {conn}");
        }));
    }
    for u in users {
        u.await.unwrap();
    }
    logln!("★ '正在建立连接'只打印一次，3 个任务几乎同时拿到同一个连接");
}
