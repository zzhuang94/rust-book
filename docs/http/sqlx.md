# 异步 SQL 数据库

> 代码：`code/http-sqlx/src/main.rs`　运行：`cargo run -p http-sqlx`（需先起 Postgres，见下一节）

一个 Postgres 版 TODO 接口：查列表 / 新增 / 翻转状态——覆盖连接池、结构体映射、绑定参数、  
404 处理、事务，外加 [《通用错误处理》](../lang/error-handling.md) thiserror 的首次实战。

----

# 先起一个 Postgres

```bash
docker run --rm -e POSTGRES_PASSWORD=postgres -p 5432:5432 postgres:16
# 服务默认连 postgres://postgres:postgres@127.0.0.1:5432/postgres，可用 DATABASE_URL 覆盖
```

```bash
cargo run -p http-sqlx
# 另开终端：
curl -X POST http://127.0.0.1:7080/todos -H 'content-type: application/json' -d '{"title":"学 sqlx"}'
curl http://127.0.0.1:7080/todos
curl -X POST http://127.0.0.1:7080/todos/1/toggle
curl -X POST http://127.0.0.1:7080/todos/999/toggle    # 404
```

----

# 数据库访问选型

> Rust 的「数据库访问」长什么样？两大流派，本课选 SQL 优先的 sqlx。

| 流派 | 代表 | Go 对照 | 一句话 |
| --- | --- | --- | --- |
| SQL 优先 | **sqlx**（本课） | database/sql + jmoiron/sqlx | 你写 SQL，它管连接池/映射/异步 |
| 查询构建器/ORM | SeaORM、diesel | GORM / ent | 用 Rust 代码生成 SQL |

本课选 sqlx 的理由和你在 Go 里用 jmoiron/sqlx 的理由一样：**SQL 就是最好的查询语言**，  
学习成本最低、行为最透明；而且 sqlx 是纯异步的，和 Tokio/axum 无缝。

----

# PgPool 是连接池

> `PgPool` ≈ Go 的 `*sql.DB`：它是池不是单连接，全局建一个、到处共享。

```rust
let pool = PgPoolOptions::new()
    .max_connections(5)              // ≈ db.SetMaxOpenConns(5)
    .connect(&url)
    .await?;
```

- 语义和 Go 的 `*sql.DB` 完全对齐：**它是池不是单连接**，全局建一个、到处共享；
- `PgPool` 是 Clone 的（内部 Arc， [《共享状态：Arc / RwLock》](../async/shared-state.md) 的老套路）——所以能直接 `.with_state(pool)` 当 axum 状态，  
  每个 handler clone 一个句柄用；
- 每次 `execute/fetch_xxx` 自动从池里借连接、用完自动还（RAII，见 [《所有权与借用》](../lang/ownership.md)）——没有 Go 里偶尔忘还 rows.Close() 导致连接泄漏的坑。

----

# 查询三板斧

> `query_as` + `bind` + `fetch_*` 三板斧，覆盖日常查询。

**行 → 结构体：FromRow**

```rust
#[derive(Serialize, sqlx::FromRow)]
struct Todo { id: i64, title: String, done: bool }

let todos = sqlx::query_as::<_, Todo>("SELECT id, title, done FROM todos ORDER BY id")
    .fetch_all(&pool)
    .await?;
```

`FromRow` 派生宏按 **列名 = 字段名** 自动填充 ≈ Go sqlx 的 StructScan；列名不一致用 `#[sqlx(rename = "...")]`。  
类型映射要点：`BIGSERIAL/BIGINT → i64`、`TEXT → String`、`BOOLEAN → bool`、  
可空列 → `Option<T>`（NULL 即 None——**数据库的 NULL 在 Rust 里是 Option，  
不是零值**，Go 里 sql.NullString 那套别扭东西不存在）。

**绑定参数：$1 $2，注入免疫**

```rust
sqlx::query_as::<_, Todo>("INSERT INTO todos (title) VALUES ($1) RETURNING id, title, done")
    .bind(&body.title)     // 依次填 $1、$2…
    .fetch_one(&pool)
    .await?;
```

值单独传输、永不拼进 SQL 字符串——SQL 注入天然免疫（纪律同款：**永远 bind，绝不 format! 拼 SQL**）。  
Postgres 用 `$1`，MySQL/SQLite 用 `?`。

**fetch 家族：按「期望几行」选**

| 方法 | 返回 | 对照 Go | 用在 |
| --- | --- | --- | --- |
| `fetch_all` | `Vec<T>` | Select 循环 Scan | 列表 |
| `fetch_one` | `T`（0 行是 Err） | QueryRow（ErrNoRows） | 必然存在的行 |
| `fetch_optional` | `Option<T>` | QueryRow + 判 ErrNoRows | **"可能没有"是正常分支** |
| `execute` | 影响行数 | Exec | 无返回行的写操作 |

代码里 toggle 的 404 就是 `fetch_optional + ok_or(AppError::NotFound)`——和 [《接入 Redis》](redis.md) 的 `Option<String>` 一个思想：  
**「没找到」走 None，「数据库坏了」走 Err**，两条路在类型上分开（Go 里都挤在 err 里，靠 `errors.Is(err, sql.ErrNoRows)` 区分）。

顺带欣赏 `RETURNING`：Postgres 的插入/更新直接返回整行，一次往返拿回带 id 的结果——Go 里常见的「Exec 完再 LastInsertId 再查一次」三步并一步。

----

# thiserror 建模错误

> [《通用错误处理》](../lang/error-handling.md) thiserror 的首次实战——对比 [《接入 Redis》](redis.md) 手写的 AppError，  
> 模板被两个属性顶掉了。

```rust
#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("数据库错误")]
    Db(#[from] sqlx::Error),      // #[from] 接通 `?`
    #[error("todo {0} 不存在")]
    NotFound(i64),
}

impl IntoResponse for AppError { ... }   // 数据库错误 → 500（细节只进日志！）；NotFound → 404
```

对比 [《接入 Redis》](redis.md) 手写的 AppError：Display 和 From 两大块模板被 `#[error]`/`#[from]` 两个属性顶掉了——这就是 [《通用错误处理》](../lang/error-handling.md) 说的「宏替你写模板」。  
另注意一个安全细节：**Db 错误的原文只进日志，不回给客户端**（连接串、表结构都可能在错误信息里）。

----

# 事务不能只转一半

> 转账要么全成、要么全不成。sqlx 的事务靠 RAII 兜底：中途任何 `?` 提前返回都自动回滚。

```rust
let mut tx = pool.begin().await?;                     // ≈ tx, _ := db.Begin()

sqlx::query("UPDATE accounts SET balance = balance - $1 WHERE id = $2")
    .bind(100).bind(from_id)
    .execute(&mut *tx)          // ★ 注意：执行器传 &mut *tx，不是 &pool！
    .await?;
sqlx::query("UPDATE accounts SET balance = balance + $1 WHERE id = $2")
    .bind(100).bind(to_id)
    .execute(&mut *tx)
    .await?;

tx.commit().await?;             // ≈ tx.Commit()
// 不 commit 就走了？tx 被 drop 时自动回滚 —— RAII 又一次兜底：
// 中途任何一个 `?` 提前返回，事务自动回滚，不存在"忘了 Rollback"（Go 要 defer tx.Rollback() 自觉）
```

----

# 编译期 SQL 校验

> sqlx 最出名的旗舰能力：`query!` 宏家族在 **编译时** 连上数据库校验你的 SQL。本课未用，但必须知道。

```rust
let todos = sqlx::query_as!(Todo, "SELECT id, title, done FROM todos")  // 注意叹号
    .fetch_all(&pool).await?;
```

它在 **编译时连上数据库**（读 DATABASE_URL），把你的 SQL 发给数据库做语法/表名/列名/类型校验——**SQL 写错、  
列类型对不上 = 编译错误**。Go 世界没有任何等价物（sqlc 代码生成算最接近的）。

本课代码用的是运行时 API（`query_as` 不带叹号），因为宏版要求 **编译机能连到数据库** ——对教学工作区（你在 Windows 看代码、  
Linux 编译）不友好。生产项目的标准解法是 `cargo sqlx prepare` 生成离线缓存（`.sqlx/` 目录进 git），  
CI 无数据库也能编译。想升级时按这条路走。

----

# migrations 一瞥

> 生产必备，本课从简。本课用 `CREATE TABLE IF NOT EXISTS` 图省事；正式项目用 sqlx-cli 管理版本化迁移。

```bash
cargo install sqlx-cli
sqlx migrate add create_todos     # 生成 migrations/xxx_create_todos.sql，写 DDL
sqlx migrate run                  # 应用；程序里也可 sqlx::migrate!().run(&pool).await
```

≈ Go 生态的 golang-migrate/goose，概念一一对应（版本表、幂等、up/down）。

----

# 动手实验

1. **跑通全流程**：走一遍上面的 curl；`docker exec -it <容器> psql -U postgres -c 'select * from todos;'` 从数据库侧验证；
2. **触发 500 与 404**：停掉 Postgres 容器再 curl（观察 500 且响应体不含连接串细节，  
   日志里才有）；toggle 一个不存在的 id（404）；
3. **加一个 DELETE 接口**：`DELETE /todos/{id}`，用 `execute` + `rows_affected()` 判断 0 行 → 404——练 fetch 家族之外的第四板斧；
4. **写一个事务接口**：`POST /todos/{id}/duplicate`——同一事务里读原行 + 插新行；  
   中途人为 `?` 失败（bind 个坏值）验证自动回滚；
5. **可空列**：给表加 `note TEXT`（可空），Todo 加 `note: Option<String>`——体会 NULL ↔ None 的直接映射。

----

# 三句话带走

1. **PgPool ≈ \*sql.DB**（池、全局一个、Clone 共享），借还全自动；查询三板斧 `query_as::<_, T>` + `bind($1)` + `fetch_all/one/optional`——「可能没有」走 Option，  
   永远 bind 绝不拼 SQL。
2. **事务靠 RAII 兜底**：`begin` 后中途任何 `?` 提前返回都自动回滚，执行器记得传 `&mut *tx`；  
   错误用 thiserror 建模，数据库错误细节只进日志。
3. sqlx 的旗舰是 **`query!` 编译期 SQL 校验**（列名类型错 = 编译错误，Go 无等价物）；  
   本课用运行时 API 保离线可编译，生产可经 `sqlx prepare` 升级；表结构管理用 migrations。

----

# 附：本课生词表

- **`PgPool` / `PgPoolOptions`** ——Postgres 连接池及其构造器；`max_connections` ≈ SetMaxOpenConns；  
  Clone 即共享。
- **`sqlx::FromRow`（derive）** ——查询行 → 结构体的映射，按列名对字段名；改名 `#[sqlx(rename)]`；  
  可空列配 `Option<T>`。
- **`query` / `query_as::<_, T>`** ——不映射/映射到 T 的运行时查询构造；`::<_, T>` 的 `_` 让数据库类型自动推断。
- **`.bind(v)`** ——按顺序填 `$1/$2…` 占位符；值走独立通道，注入免疫。
- **`fetch_all / fetch_one / fetch_optional / execute`** ——Vec / 必有一行 / Option / 影响行数（`rows_affected()`）。
- **`RETURNING`** ——Postgres 方言：写操作直接返回行，省一次往返。
- **`pool.begin()` / `tx.commit()` / drop 即回滚** ——事务三件套；执行器传 `&mut *tx`。
- **`query!` / `query_as!` / `cargo sqlx prepare`** ——编译期校验宏家族与离线缓存方案（本课选读）。
- **`sqlx migrate`** ——版本化迁移 ≈ golang-migrate；程序内 `sqlx::migrate!()` 可自动应用。
