//! Lesson 13 —— sqlx：异步 SQL 数据库（Postgres 版 TODO 接口）。
//!
//! 对照 Go 的心智映射：
//!   PgPool             ≈ *sql.DB（连接池，全局一个到处共享）
//!   query_as + FromRow ≈ sqlx.StructScan / rows.Scan 进 struct
//!   $1/$2 绑定参数      ≈ database/sql 的占位符（防注入）
//!   pool.begin/commit  ≈ db.Begin / tx.Commit
//!
//! 先起数据库：docker run --rm -e POSTGRES_PASSWORD=postgres -p 5432:5432 postgres:16
//! 运行：cargo run -p http-sqlx   （可用 DATABASE_URL 覆盖默认连接串）
//! 试：
//!   curl -X POST http://127.0.0.1:7080/todos -H 'content-type: application/json' -d '{"title":"学 sqlx"}'
//!   curl http://127.0.0.1:7080/todos
//!   curl -X POST http://127.0.0.1:7080/todos/1/toggle

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use labkit::logln;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

// ---------- 数据模型 ----------

/// FromRow：sqlx 的派生宏，把查询结果的一行按【列名 = 字段名】填进结构体。
/// ≈ Go 里 sqlx.StructScan 的 `db:"title"` tag 体系（这里默认同名直接对上）。
#[derive(Serialize, sqlx::FromRow)]
struct Todo {
    id: i64,
    title: String,
    done: bool,
}

#[derive(Deserialize)]
struct NewTodo {
    title: String,
}

// ---------- 错误处理：00h 课 thiserror 的实战 ----------

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("数据库错误")]
    Db(#[from] sqlx::Error), // #[from]：让 `?` 自动把 sqlx::Error 装进来（00h §3.2）

    #[error("todo {0} 不存在")]
    NotFound(i64),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::Db(e) => {
                logln!("数据库错误: {e}"); // 细节进日志，不外泄给客户端
                StatusCode::INTERNAL_SERVER_ERROR
            }
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
        };
        (status, self.to_string()).into_response() // to_string 来自 #[error("...")] 的 Display
    }
}

// ---------- 启动 ----------

#[tokio::main]
async fn main() {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/postgres".to_string());

    // 连接池：设好最大连接数。池是 Clone 的（内部 Arc），直接当 axum State。
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("无法连接 Postgres，请确认 docker 容器已启动、DATABASE_URL 正确");
    logln!("已连接 Postgres: {url}");

    // 建表（幂等）。正式项目用 sqlx-cli 的 migrations 管理，见文档 §7。
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS todos (
            id    BIGSERIAL PRIMARY KEY,
            title TEXT NOT NULL,
            done  BOOLEAN NOT NULL DEFAULT FALSE
        )",
    )
    .execute(&pool)
    .await
    .expect("建表失败");

    let app = Router::new()
        .route("/todos", get(list_todos).post(create_todo))
        .route("/todos/{id}/toggle", post(toggle_todo))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:7080").await.unwrap();
    logln!("HTTP 服务已启动：http://127.0.0.1:7080");
    axum::serve(listener, app).await.unwrap();
}

// ---------- handlers ----------

/// GET /todos —— 查列表。
/// query_as::<_, Todo>：执行 SQL，把每行按 FromRow 填成 Todo。
async fn list_todos(State(pool): State<PgPool>) -> Result<Json<Vec<Todo>>, AppError> {
    let todos = sqlx::query_as::<_, Todo>("SELECT id, title, done FROM todos ORDER BY id")
        .fetch_all(&pool) // fetch_all → Vec<Todo>；姐妹们：fetch_one / fetch_optional
        .await?; // sqlx::Error 经 #[from] 自动变 AppError → 500
    Ok(Json(todos))
}

/// POST /todos —— 新增，返回带 id 的完整行。
/// ★ $1 是绑定参数：值单独传输、永不拼进 SQL 字符串 —— SQL 注入在这里天然免疫。
async fn create_todo(
    State(pool): State<PgPool>,
    Json(body): Json<NewTodo>,
) -> Result<(StatusCode, Json<Todo>), AppError> {
    let todo = sqlx::query_as::<_, Todo>(
        "INSERT INTO todos (title) VALUES ($1) RETURNING id, title, done",
    )
    .bind(&body.title) // 依次填 $1、$2…
    .fetch_one(&pool)
    .await?;
    Ok((StatusCode::CREATED, Json(todo)))
}

/// POST /todos/{id}/toggle —— 翻转完成状态；不存在返回 404。
/// fetch_optional：0 行 → None（"没找到"是正常分支，不是错误——对照 08 课 redis 的 Option）。
async fn toggle_todo(
    State(pool): State<PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<Todo>, AppError> {
    let todo = sqlx::query_as::<_, Todo>(
        "UPDATE todos SET done = NOT done WHERE id = $1 RETURNING id, title, done",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await?
    .ok_or(AppError::NotFound(id))?; // None → 404

    Ok(Json(todo))
}
