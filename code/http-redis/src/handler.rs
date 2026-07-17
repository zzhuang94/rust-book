//! 读写 Redis 的处理函数。
//!
//! `use redis::AsyncCommands;` 引入后，连接对象就有了 .get / .set / .incr / .set_ex 等
//! 类型化的异步方法。对照 Go go-redis 的 rdb.Get / rdb.Set / rdb.Incr。

use axum::extract::{Path, State};
use axum::Json;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::state::AppState;

/// 统一给 key 加个命名空间前缀，避免和别的数据撞车。
fn redis_key(key: &str) -> String {
    format!("lab:{key}")
}

/// GET /  —— 健康检查。
pub async fn health() -> &'static str {
    "ok"
}

/// GET /counter —— 对一个计数器做原子自增，返回新值。
///
/// INCR 是 Redis 的原子操作：即使多个实例并发打这个接口，也不会丢更新。
/// 对照 Go：rdb.Incr(ctx, "lab:counter").Result()。
pub async fn incr_counter(
    State(state): State<AppState>,
) -> Result<Json<CounterResp>, AppError> {
    // clone 一份连接句柄（共享同一底层多路复用连接），方法需要 &mut self。
    let mut con = state.redis.clone();
    let counter: i64 = con.incr("lab:counter", 1).await?; // 出错 → `?` → AppError → 500
    Ok(Json(CounterResp { counter }))
}

/// GET /kv/{key} —— 读一个字符串；不存在返回 404。
/// 对照 Go：val, err := rdb.Get(ctx, key).Result()  // err == redis.Nil 表示不存在
pub async fn get_kv(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<KvResp>, AppError> {
    let mut con = state.redis.clone();
    // 用 Option<String> 接收：键不存在时得到 None，而不是报错。
    let value: Option<String> = con.get(redis_key(&key)).await?;
    match value {
        Some(value) => Ok(Json(KvResp { key, value })),
        None => Err(AppError::NotFound(key)),
    }
}

/// POST /kv —— 写一个字符串，可选 TTL（秒）。
/// 对照 Go：rdb.Set(ctx, key, val, ttl)  // ttl==0 表示不过期
pub async fn set_kv(
    State(state): State<AppState>,
    Json(body): Json<SetReq>,
) -> Result<Json<SetResp>, AppError> {
    let mut con = state.redis.clone();
    let key = redis_key(&body.key);

    // ★ 小白必看：redis crate 的命令方法是「泛型返回」的 —— 同一个 .get()/.set()，
    //   你在等号左边标注什么类型，它就按什么类型解析 Redis 的回包。
    //   所以写命令时必须写 `let _: () = ...`，显式告诉编译器"回包丢弃即可"；
    //   漏了类型标注会得到"cannot infer type"（无法推断类型）的编译错误 ——
    //   这是用这个库时新手最常撞的墙，撞到就回来看这条注释。
    match body.ttl_secs {
        // SETEX：写入并设置过期时间（秒）。
        Some(ttl) => {
            let _: () = con.set_ex(key, &body.value, ttl).await?;
        }
        // SET：永久写入。
        None => {
            let _: () = con.set(key, &body.value).await?;
        }
    }

    Ok(Json(SetResp {
        ok: true,
        key: body.key,
        ttl_secs: body.ttl_secs,
    }))
}

// ---- 请求 / 响应体 ----

#[derive(Serialize)]
pub struct CounterResp {
    pub counter: i64,
}

#[derive(Serialize)]
pub struct KvResp {
    pub key: String,
    pub value: String,
}

#[derive(Deserialize)]
pub struct SetReq {
    pub key: String,
    pub value: String,
    /// 可选过期时间（秒）；不传就永久。
    pub ttl_secs: Option<u64>,
}

#[derive(Serialize)]
pub struct SetResp {
    pub ok: bool,
    pub key: String,
    pub ttl_secs: Option<u64>,
}
