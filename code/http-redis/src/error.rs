//! 统一错误类型。
//!
//! 这是 axum 里做错误处理的惯用法：定义一个 AppError，为它实现 IntoResponse，
//! 并用 `From` 把底层错误(这里是 redis::RedisError)自动转进来。
//! 之后 handler 就能对任何 Redis 调用直接用 `?`：出错自动转成 AppError 再变 HTTP 响应。
//!
//! 对照 Go：相当于把 `if err != nil { return err }` + 统一的错误响应中间件，
//! 收敛成「返回 Result + `?`」——但转换是编译期、类型安全的。

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

pub enum AppError {
    /// 底层 Redis 操作失败 → 500。
    Redis(redis::RedisError),
    /// key 不存在 → 404。
    NotFound(String),
}

/// 有了这个 From，`redis_call().await?` 里的 `?` 就能把 RedisError 自动变成 AppError。
impl From<redis::RedisError> for AppError {
    fn from(err: redis::RedisError) -> Self {
        AppError::Redis(err)
    }
}

#[derive(Serialize)]
struct ErrBody {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Redis(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("redis 错误: {err}"),
            ),
            AppError::NotFound(key) => {
                (StatusCode::NOT_FOUND, format!("key 不存在: {key}"))
            }
        };
        (status, Json(ErrBody { error: message })).into_response()
    }
}
