//! Lesson 11 —— 测试。本 crate 是"被测对象"：三个模块对应三类测试场景。
//!   calc  ：纯函数        → 普通测试 + 文档测试
//!   cache ：异步 TTL 缓存 → #[tokio::test] + 时间控制（不用真等）
//!   api   ：axum 接口     → oneshot 内存调用（不开端口）
//!
//! 测试代码在 tests/ 目录（独立集成测试，本项目的约定）。
//! 跑法：cargo test -p engineering-testing

pub mod api;
pub mod cache;
pub mod calc;
