//! Lesson 03 —— 共享状态（保姆级）：从"编译器为什么拦你"一路讲到死锁。
//! 这是整个课程最核心的一课：第 04 课的 HTTP 服务 = 本课模型 + 一层网络。
//!
//! 九个场景，由浅入深：
//!   (1) Arc：先解决"共享"（只读），顺便观察引用计数
//!   (2) Send/Sync：编译器判断"能不能跨线程"的两张体检表（背景知识）
//!   (3) 想改数据？——看编译器怎么拦（注释演示），引出"内部可变性"
//!   (4) Mutex 互斥锁：最简单的"共享 + 可改"（并发累加计数器）
//!   (5) RwLock 读写锁：读多写少 →「定期写 + 高并发读」（本项目内核）
//!   (6) AtomicU64 原子量：只是计数的话，锁都不用
//!   (7) tokio::sync::Mutex：真的要"持锁跨 await"时的正确姿势
//!   (8) 死锁：交叉加锁现场复现（用超时兜底观察），再看怎么修
//!   (9) 反面教材（注释）：std 锁 guard 跨 await —— 编译器直接拒绝
//!
//! 对照 Go 总览：
//!   Arc<T>              ≈ 共享指针（Go 靠 GC，Rust 靠引用计数）
//!   Send/Sync           ≈ 无对应物！Go 靠 -race 运行时抓竞争，Rust 编译期拦
//!   Mutex<T>            ≈ sync.Mutex + 它保护的变量（合为一体！）
//!   RwLock<T>           ≈ sync.RWMutex + 变量
//!   AtomicU64           ≈ atomic.Int64
//!   tokio::sync::Mutex  ≈ 无直接对应（Go 的锁天生"可跨阻塞"）
//!
//! 运行：cargo run -p async-shared-state

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use labkit::logln;
use tokio::time::{interval, sleep, timeout};

#[tokio::main]
async fn main() {
    // =====================================================================
    // (1) Arc：先解决"共享"（只读），顺便观察引用计数
    // =====================================================================
    // 为什么不能像 Go 那样直接把变量塞给几个任务？两条路都被堵死：
    //   - 借用（&config）：spawn 的任务要求 'static，不能借栈上变量（02 课）；
    //   - 移动（move）：所有权只有一份，move 给任务 A 之后任务 B 就没得用了。
    // Arc 就是第三条路：每个任务拿一个"计数句柄"，共享同一份堆上数据。
    logln!("(1) Arc 只读共享 ------------------------------------------------------");
    let config = Arc::new(vec!["读多".to_string(), "写少".to_string()]);
    logln!("初始引用计数 = {}", Arc::strong_count(&config)); // 1

    let mut handles = Vec::new();
    for id in 0..3 {
        let config = Arc::clone(&config); // 计数 +1，不复制数据
        handles.push(tokio::spawn(async move {
            // 只读共享不需要任何锁：Arc<T> 允许多任务同时 & 读
            logln!("    任务 {id} 读到配置: {config:?}");
            sleep(Duration::from_millis(50)).await;
            // 任务结束，这个 config 句柄 drop，计数 -1
        }));
    }
    logln!("spawn 后引用计数 = {}（1 个 main + 3 个任务）", Arc::strong_count(&config));
    for h in handles {
        h.await.unwrap();
    }
    logln!("任务全部结束后引用计数 = {}（只剩 main 手里这份）", Arc::strong_count(&config));
    logln!("（计数归零的那一刻数据才释放 —— Arc 就是'手动、确定性的 GC'）\n");

    // =====================================================================
    // (2) Send/Sync：编译器判断"能不能跨线程"的两张体检表
    // =====================================================================
    // 背景问题：场景(1)里 Arc 被 spawn 到各个任务，编译器凭什么放行？
    // 靠两个「标记 trait」（marker trait：没有任何方法的空 trait，纯粹是贴标签，
    // 你最早学的 Copy 也是这一类）：
    //
    //   Send：这个值的【所有权】可以安全地搬到另一个线程去用/销毁
    //   Sync：这个值可以安全地被多个线程【同时引用】（&T 同时存在于多线程）
    //
    // 标签由编译器**自动推导**：结构体所有字段都 Send，它就自动是 Send（Sync 同理）。
    // 绝大多数类型都自动带标签；个别类型被**故意不发**标签 ——
    // 最典型的是 Rc：它的引用计数是普通整数，两个线程同时 clone 会把计数加坏
    // （少计 → 提前释放 → 悬垂指针），所以编译器禁止它跨线程。
    // Arc 的计数是原子操作，所以有标签。
    //
    // 下面用两个空泛型函数当"体检器"：只有带对应标签的类型才能通过编译。
    fn assert_send<T: Send>(_v: &T) {} // 体检：T 有 Send 标签吗？
    fn assert_sync<T: Sync>(_v: &T) {} // 体检：T 有 Sync 标签吗？

    logln!("(2) Send/Sync 体检 ----------------------------------------------------");
    let arc = Arc::new(42);
    assert_send(&arc); // ✅ 编译通过：Arc<i32> 是 Send，可以 move 进 spawn 的任务
    assert_sync(&arc); // ✅ 编译通过：Arc<i32> 是 Sync，可以被多任务同时 & 引用
    logln!("Arc<i32> 体检通过：Send ✅ Sync ✅（原子计数，跨线程安全）");

    let rc = std::rc::Rc::new(42);
    let _rc2 = rc.clone(); // Rc 在单线程里用完全没问题（非原子计数，还更快）
    logln!("Rc<i32> 单线程使用没问题，但它没有 Send/Sync 标签：");
    logln!("   （下面两行注释任选一行放开，都会得到经典报错，见源码）");
    // assert_send(&rc);
    //   ↑ ❌ error[E0277]: `Rc<i32>` cannot be sent between threads safely
    // tokio::spawn(async move { let _ = rc; });
    //   ↑ ❌ 同样的报错 —— spawn 的签名要求任务 Send（02 课讲过），
    //     编译器在这里"查体检表"，没标签就不放行。
    //
    // ★ 传染性：只要结构体里有一个字段是 Rc，整个结构体就失去 Send，
    //   包含它的任务也失去 Send —— 报错会一路指回源头。这不是刁难，
    //   是把"数据竞争"这类最难查的 bug 消灭在编译期（Go 只能靠 -race 运行时抽查）。
    logln!("");

    // =====================================================================
    // (3) 想改数据？—— 看编译器怎么拦，引出"内部可变性"
    // =====================================================================
    // 直觉写法（下面两行放开会编译失败）：
    //
    //     let shared = Arc::new(vec![1, 2, 3]);
    //     shared.push(4);   // ❌ error: cannot borrow data in an `Arc` as mutable
    //
    // 为什么？Rust 的规矩是"要么多个只读引用，要么一个可变引用"。
    // Arc 的本职是让**很多人同时持有**，那就人人只读 —— 谁都不许改。
    // 想改，就需要一个"哪怕通过只读引用也能安全修改内部"的容器，
    // 这类容器统称「内部可变性」，多线程版就是 Mutex / RwLock / Atomic 家族：
    //   Arc<Mutex<T>>  —— 修改前排队拿锁（场景(4)）
    //   Arc<RwLock<T>> —— 读共享、写独占（场景(5)）
    //   Arc<AtomicU64> —— 硬件原子指令（场景(6)）
    logln!("(3) Arc 里的数据不能直接改（见源码注释里的编译错误），");
    logln!("   解法 = 内部可变性容器：Mutex / RwLock / Atomic，往下看\n");

    // =====================================================================
    // (4) Mutex：最简单的"共享 + 可改"
    // =====================================================================
    // 经典题目：4 个任务并发给同一个计数器各 +1000。
    // 对照 Go：
    //     var mu sync.Mutex; var n int
    //     mu.Lock(); n++; mu.Unlock()
    // 关键差异：Go 的锁和数据是两个变量，全靠自觉配对；
    // Rust 的 Mutex<T> 把数据**装在锁里面**，不 lock 就拿不到数据 ——
    // "忘了加锁"在语法上就不可能发生。
    // （顺带一提：Mutex<T> 之所以是 Sync —— 能被多任务共享 ——
    //   正是因为所有访问都要过锁。锁把"不可共享的改"变成了"可共享的改"。）
    logln!("(4) Mutex 并发累加 ----------------------------------------------------");
    let counter = Arc::new(Mutex::new(0u64));
    let mut handles = Vec::new();
    for _ in 0..4 {
        let counter = Arc::clone(&counter);
        handles.push(tokio::spawn(async move {
            for _ in 0..1000 {
                // lock() 返回 guard（"通行证"）：
                //   - 持有 guard 期间独占数据（别人 lock 会排队）；
                //   - guard 在语句末尾 drop → 自动解锁（RAII，不需要 defer Unlock）。
                // unwrap()：锁"中毒"（持锁者 panic 过）时快速失败，见文档。
                *counter.lock().unwrap() += 1;
            }
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    logln!("4 个任务各 +1000，结果 = {}（精确 4000，一次不丢）", *counter.lock().unwrap());
    logln!("（如果不加锁裸改 u64？Rust 直接不给编译；Go 能编译，结果随机少）\n");

    // =====================================================================
    // (5) RwLock：读多写少 →「定期写 + 高并发读」（本项目内核）
    // =====================================================================
    // Mutex 的问题：读也互斥 —— 100 个读者也得排队，太浪费。
    // RwLock 区分两种锁：
    //   read()  读锁：可以很多读者**同时**持有，互不阻塞；
    //   write() 写锁：独占，写时挡住所有人。
    // 正好匹配"一个写者定期刷新 + 无数读者高频读"的形态。
    logln!("(5) RwLock 定期写 + 并发读 ---------------------------------------------");
    // 共享的数据：(版本号, 一批数字)
    let data = Arc::new(RwLock::new((0u64, vec![0i64; 3])));

    // --- 写者：每 100ms 整体替换一次 ---
    {
        let data = Arc::clone(&data);
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(100));
            let mut version = 0u64;
            loop {
                ticker.tick().await;
                version += 1;
                // ★ 套路：重活（构造新数据）在锁**外**做完……
                let fresh = (version, vec![version as i64; 3]);
                // ……进锁只做一次赋值。guard 是临时值，本语句结束即 drop = 解锁。
                *data.write().unwrap() = fresh;
            }
        });
    }

    // --- 读者：3 个任务并发读，每人读 5 次 ---
    let mut readers = Vec::new();
    for id in 0..3 {
        let data = Arc::clone(&data);
        readers.push(tokio::spawn(async move {
            for _ in 0..5 {
                // ★ 用 { } 小作用域圈住持锁时间：
                //   进锁 → clone 出需要的数据 → 作用域结束 guard 自动 drop = 解锁。
                //   之后再慢慢用数据、再 .await，都与锁无关。
                let (ver, items) = {
                    let guard = data.read().unwrap(); // 读锁：读者之间互不阻塞
                    (guard.0, guard.1.clone())
                }; // ← guard 在此 drop，锁已释放

                logln!("    读者 {id} 看到 version={ver}, items={items:?}");
                sleep(Duration::from_millis(40)).await; // 此刻没有持锁
            }
        }));
    }
    for r in readers {
        r.await.unwrap();
    }
    logln!("（version 在涨 = 写者在后台工作；读者时间戳交错 = 并发读）\n");

    // =====================================================================
    // (6) AtomicU64：只是计数的话，锁都不用
    // =====================================================================
    // 场景(4)的计数器用 Mutex 有点"杀鸡用牛刀"：
    // 单个整数的加减，硬件本身就有原子指令，不需要锁的排队开销。
    // 对照 Go 的 atomic.AddInt64 / atomic.LoadInt64。
    logln!("(6) 原子计数器 --------------------------------------------------------");
    let counter = Arc::new(AtomicU64::new(0));
    let mut handles = Vec::new();
    for _ in 0..4 {
        let counter = Arc::clone(&counter);
        handles.push(tokio::spawn(async move {
            for _ in 0..1000 {
                // fetch_add：原子的"读-加-写"三合一，一条硬件指令级保证。
                // Relaxed：最宽松内存序，独立计数器够用（拿不准用 SeqCst）。
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    logln!("原子累加结果 = {}（同样精确 4000，但没有锁）", counter.load(Ordering::Relaxed));
    logln!("（想亲手制造竞态？把 fetch_add 拆成 load 再 store 试试 —— 见动手实验）\n");

    // =====================================================================
    // (7) tokio::sync::Mutex：真的要"持锁跨 await"时的正确姿势
    // =====================================================================
    // 前面全用 std 锁，因为临界区都不含 .await。
    // 但有的需求就是要"持锁期间做异步操作"，比如：独占一个连接发请求、
    // 顺序地往共享日志里写"开始/结束"两条配对记录（中间隔着一次异步 IO）。
    // std 锁的 guard 不是 Send，跨 await 编译不过（场景(9)）；
    // tokio::sync::Mutex 的 guard 可以跨 await —— lock() 本身也是异步的。
    logln!("(7) tokio 异步锁：合法地持锁跨 await -----------------------------------");
    let journal = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
    let mut handles = Vec::new();
    for id in 1..=2 {
        let journal = Arc::clone(&journal);
        handles.push(tokio::spawn(async move {
            // 注意 lock() 后面是 .await（异步排队），不是 .unwrap()（无中毒概念）
            let mut guard = journal.lock().await;
            logln!("    任务 {id} 拿到锁，写'开始'，然后持锁做 300ms 异步 IO");
            guard.push(format!("任务{id}-开始"));
            sleep(Duration::from_millis(300)).await; // ★ 持锁跨 await：tokio 锁允许
            guard.push(format!("任务{id}-结束"));
            logln!("    任务 {id} 写'结束'，放锁");
        })); // guard 在任务结尾 drop → 解锁
    }
    for h in handles {
        h.await.unwrap();
    }
    logln!("日志内容: {:?}", journal.lock().await);
    logln!("★ 两个任务的'开始/结束'各自相邻、不交错 —— 锁保证了配对完整性；");
    logln!("  代价：任务 2 干等了任务 1 整整 300ms。异步锁能用，但临界区仍要尽量短\n");

    // =====================================================================
    // (8) 死锁：交叉加锁现场复现，再看怎么修
    // =====================================================================
    // 经典配方（还记得你见过的 MUTEX1/MUTEX2 例子吗）：
    //   任务甲：锁 A → 睡一会 → 想锁 B
    //   任务乙：锁 B → 睡一会 → 想锁 A
    // 甲攥着 A 等 B，乙攥着 B 等 A —— 谁也不放手，永远僵住。
    // 这里用 timeout 兜底，让你能"看到"死锁而程序不会真挂死。
    logln!("(8) 死锁复现 ----------------------------------------------------------");
    let lock_a = Arc::new(tokio::sync::Mutex::new("资源A"));
    let lock_b = Arc::new(tokio::sync::Mutex::new("资源B"));

    let mut h1 = {
        let (a, b) = (Arc::clone(&lock_a), Arc::clone(&lock_b));
        tokio::spawn(async move {
            let _ga = a.lock().await;
            logln!("    任务甲：锁住了 A，准备去锁 B");
            sleep(Duration::from_millis(50)).await; // 给乙留出锁 B 的时间
            let _gb = b.lock().await; // ← 永远等不到（B 在乙手里）
            logln!("    任务甲：拿到了 B（死锁时这行不会出现）");
        })
    };
    let mut h2 = {
        let (a, b) = (Arc::clone(&lock_a), Arc::clone(&lock_b));
        tokio::spawn(async move {
            let _gb = b.lock().await;
            logln!("    任务乙：锁住了 B，准备去锁 A");
            sleep(Duration::from_millis(50)).await;
            let _ga = a.lock().await; // ← 永远等不到（A 在甲手里）
            logln!("    任务乙：拿到了 A（死锁时这行不会出现）");
        })
    };
    // 用 timeout 观察：1 秒还没完成 = 死锁了。
    // （&mut h1：借用着等，超时后句柄还在我们手里，下面还要用它 abort）
    match timeout(Duration::from_millis(1000), async {
        let _ = (&mut h1).await;
        let _ = (&mut h2).await;
    })
    .await
    {
        Ok(_) => logln!("两个任务都完成了？（不该发生）"),
        Err(_) => logln!("★ 1 秒超时：死锁确认！甲攥着A等B，乙攥着B等A"),
    }
    // 注意：编译器**不防死锁**（它防的是数据竞争）。Go 的 -race 同样查不出死锁。

    // 甲乙还僵着、各占一把锁，得先清场：abort 强制取消任务 ——
    // 任务的 future 被 drop，它持有的 guard 一并 drop → A、B 被释放。
    // （又是第 01 课那条："丢弃 future = 取消"，配合 RAII 连锁都自动还了）
    h1.abort();
    h2.abort();
    logln!("已 abort 甲乙，两把锁随任务销毁而释放");

    // --- 修复：所有任务按同一顺序加锁（先 A 后 B），环路不成立 ---
    logln!("--- 修复版：统一加锁顺序（都先 A 后 B）---");
    let mut fixed = Vec::new();
    for name in ["丙", "丁"] {
        let (a, b) = (Arc::clone(&lock_a), Arc::clone(&lock_b));
        fixed.push(tokio::spawn(async move {
            // 想拿多把锁？全项目约定一个固定顺序，人人遵守。
            let _ga = a.lock().await;
            logln!("    任务{name}：锁住 A");
            sleep(Duration::from_millis(50)).await;
            let _gb = b.lock().await;
            logln!("    任务{name}：锁住 B，两把都到手，干活，放锁");
        }));
    }
    match timeout(Duration::from_millis(1000), async {
        for f in fixed {
            let _ = f.await;
        }
    })
    .await
    {
        Ok(_) => logln!("★ 修复版顺利完成：统一加锁顺序后，环路不可能形成\n"),
        Err(_) => logln!("修复版也超时了？不该发生，检查 abort 是否生效\n"),
    }

    // =====================================================================
    // (9) 反面教材（注释）：std 锁 guard 跨 await —— 编译器直接拒绝
    // =====================================================================
    // 把场景(5)读者改成下面这样（故意持 std 读锁去 .await）：
    //
    //     let guard = data.read().unwrap();          // 拿到 std 读锁
    //     sleep(Duration::from_millis(10)).await;    // ❌ 攥着锁 .await
    //     logln!("{}", guard.0);
    //
    // 编译报错（原文）：
    //     error: future cannot be sent between threads safely
    //     note:  `std::sync::RwLockReadGuard<'_, ...>` is not `Send`
    //
    // 人话翻译（现在你有场景(2)的背景，能真正读懂它了）：
    // ".await 时任务可能被挪到别的线程继续跑，而 std 锁的 guard 没有 Send 标签、
    //  不允许跨线程携带，所以整个任务不满足 spawn 的 Send 要求"。
    // 这是编译器在救你：持 std 锁跨 await 轻则堵死别人、重则死锁。
    // 两条出路：
    //   a) 像场景(5)那样用 { } 在 await 前把锁放掉（首选，99% 的场景够用）；
    //   b) 真的要持锁跨 await → 用场景(7)的 tokio::sync::Mutex/RwLock。
    logln!("(9) std 锁跨 await 的反面教材见源码注释（编译器拦截，跑都跑不起来）");
    logln!("\n第 03 课演示结束。下一课：把「定期写 + 并发读」装进 HTTP 服务");
}
