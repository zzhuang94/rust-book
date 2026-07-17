//! Lesson 04 —— 简易高并发 HTTP 服务的核心库。
//!
//! 三个模块各司其职：
//!   - state   ：全局共享状态（内存数据 + 统计），对应 Gin 里挂在 handler 上的单例。
//!   - updater ：后台任务，定期整体刷新内存数据。
//!   - handler ：各个 HTTP 处理函数，对应 Gin 的 HandlerFunc。

pub mod handler;
pub mod state;
pub mod updater;
