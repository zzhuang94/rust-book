//! Lesson 06 —— 用 ArcSwap 把「读」做成完全无锁。
//! 功能和第 04 课一模一样（定期更新 + 高并发读），只把状态层从
//! `Arc<RwLock<Snapshot>>` 换成 `ArcSwap<Snapshot>`，方便直接对比。

pub mod handler;
pub mod state;
pub mod updater;
