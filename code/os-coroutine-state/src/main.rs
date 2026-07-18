//! 手写一个极简状态机：暂停 = 返回，恢复 = 再调一次
//!
//! 配套文档：docs/concurrency/os-basics.md 「状态机是什么」
//! 运行：cargo run -p os-coroutine-state（先 cd code）
//!
//! docs 里的核心论断：Rust 的 async 任务在编译后就是一台状态机——
//! "执行到哪了"不再藏在某个线程的寄存器/栈里，而是变成一个普通的枚举值。
//! "暂停"就是 poll() 函数返回一次；"恢复"就是外部再调一次 poll()。
//! 这份代码完全不用 async/await，手写一个"打招呼"状态机，亲手实现这套心智模型——
//! 接下来看真正的 `Future::poll` 时，会发现它就是编译器帮你自动生成的同一种东西，
//! 只是字段和状态是编译器推导出来的，不是你手写的。

use std::time::{Duration, Instant};

use labkit::logln;

/// "打招呼"这个过程的三个状态，对应 docs 里的示意：
///   还没开始 → 正在等定时器（带着"什么时候开始等的 + 要等多久"这两个中间数据） → 已完成
enum GreetState {
    NotStarted { need: Duration },
    WaitingTimer { started_at: Instant, need: Duration },
    Done,
}

/// poll 一次的结果：Pending = 还没好，继续等；Ready(T) = 好了，这是结果。
/// 这两个名字故意照抄 std::task::Poll 的命名——真正的 Future::poll 返回的就是它。
enum PollResult<T> {
    Pending,
    Ready(T),
}

/// 一个"打招呼"状态机：name 是它随身带着的数据，state 是"执行到哪了"。
/// 注意 Greet 本身只是一个普通的结构体（几十字节），可以 move、可以放进 Vec——
/// 这正是"状态机是普通数据"这句话的字面意思。
struct Greet {
    name: String,
    state: GreetState,
}

impl Greet {
    fn new(name: &str, wait: Duration) -> Self {
        Greet { name: name.to_string(), state: GreetState::NotStarted { need: wait } }
    }

    /// 被"推一下"：外部调一次这个函数，状态机就尝试往前走一步。
    ///
    /// 关键点：**每次调用都是一次普通函数调用/返回，没有创建线程、没有保存寄存器**——
    /// 这就是"用户态任务切换是纳秒级"的字面意思，对比线程上下文切换的微秒级。
    fn poll(&mut self) -> PollResult<String> {
        match self.state {
            GreetState::NotStarted { need } => {
                logln!("  [{}] 还没开始 -> 转成「正在等定时器」（需要 {need:?}）", self.name);
                self.state = GreetState::WaitingTimer { started_at: Instant::now(), need };
                PollResult::Pending // 暂停：把"从哪继续"存进了 state，函数直接返回
            }
            GreetState::WaitingTimer { started_at, need } => {
                if started_at.elapsed() >= need {
                    let msg = format!("你好，{}！定时器到点了", self.name);
                    logln!("  [{}] 定时器到了 -> 转成「已完成」", self.name);
                    self.state = GreetState::Done;
                    PollResult::Ready(msg)
                } else {
                    logln!(
                        "  [{}] 还在等定时器（已过 {:?}/{need:?}）-> 仍是 Pending",
                        self.name,
                        started_at.elapsed()
                    );
                    PollResult::Pending
                }
            }
            GreetState::Done => PollResult::Ready(format!("{} 早就问候完了", self.name)),
        }
    }
}

fn main() {
    logln!("--- 手写状态机：poll 几次，观察「暂停=返回，恢复=再调一次」 ---");

    // 两台状态机，各自"需要等待"的时长不同，模拟两个进度不一样的任务。
    let mut fast = Greet::new("小明", Duration::from_millis(30));
    let mut slow = Greet::new("小红", Duration::from_millis(90));

    // 外部用一个简单循环"轮流推一下"两台状态机——这正是异步运行时 executor 干的事：
    // 它不知道、也不关心内部实现，只管反复调用 poll，直到状态机说 Ready。
    let mut fast_done = false;
    let mut slow_done = false;
    let mut round = 0;

    while !fast_done || !slow_done {
        round += 1;
        logln!("=== 第 {round} 轮 poll ===");

        if !fast_done {
            if let PollResult::Ready(msg) = fast.poll() {
                logln!("★ 小明 完成: {msg}");
                fast_done = true;
            }
        }
        if !slow_done {
            if let PollResult::Ready(msg) = slow.poll() {
                logln!("★ 小红 完成: {msg}");
                slow_done = true;
            }
        }

        // 模拟"这段时间让别的活干着，等一下再来推"——真正的运行时靠 Waker 决定何时再推，
        // 这里简化成固定间隔 sleep，纯粹是教学 demo，不代表真实调度策略。
        std::thread::sleep(Duration::from_millis(20));
    }

    logln!("两台状态机都跑完了，总共推了 {round} 轮");
    logln!("★ 对照 Future：async fn 编译后就是这样一台状态机，每个 .await 是一个状态边界，");
    logln!("  poll() 就是「推一下」，Waker 负责告诉执行器「什么时候值得再推」而不是傻等");
}
