//! Lesson 08 —— 接入 Redis 异步读写。
//!
//! 前几课的「内存数据」是单进程私有的；换成 Redis 后，数据变成**跨进程/跨实例共享**，
//! 这也是把服务水平扩容时的常见做法（多个实例读写同一个 Redis）。
//!
//! 模块划分：
//!   - state   ：持有 Redis 连接（ConnectionManager，可 Clone、自带重连）。
//!   - error   ：统一的 AppError，演示用 `?` 传播错误并转成 HTTP 响应。
//!   - handler ：读写 Redis 的各个接口。

pub mod error;
pub mod handler;
pub mod state;
