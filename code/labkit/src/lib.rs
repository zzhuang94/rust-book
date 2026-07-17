//! 课程通用工具 crate。
//!
//! 目前只提供一个东西：带毫秒时间戳的日志宏 `logln!`。
//! 用法和标准库的 `println!` 完全一样，只是每行前面自动加上当前本地时间前缀，
//! 形如 `[2026-07-11 22:22:01.123] 你的消息`，方便观察异步任务的时序。
//!
//! 用法：
//! ```ignore
//! use labkit::logln;
//! logln!("say({name}) 开始");
//! ```

/// 返回当前本地时间字符串，精确到毫秒，格式 `2026-07-11 22:22:01.123`。
///
/// 对照 Go：`time.Now().Format("2006-01-02 15:04:05.000")`。
/// chrono 的格式符：`%Y-%m-%d`=日期，`%H:%M:%S`=时分秒，`%.3f`=`.123`(点+3 位毫秒)。
pub fn now() -> String {
    chrono::Local::now()
        .format("%Y-%m-%d %H:%M:%S%.3f")
        .to_string()
}

/// 和 `println!` 用法一致，但每行自动加毫秒时间戳前缀。
///
/// 用一次 `println!` 调用输出整行（而不是分两次 print），避免多任务并发时半行交错。
#[macro_export]
macro_rules! logln {
    ($($arg:tt)*) => {
        println!("[{}] {}", $crate::now(), ::std::format_args!($($arg)*))
    };
}
