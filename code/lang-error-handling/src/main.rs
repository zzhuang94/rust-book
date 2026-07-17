//! 语言地基 · 通用错误处理 —— 可运行示例
//!
//! 配套文档：docs/lang/error-handling.md
//! 运行：cargo run -p lang-error-handling（先 cd code）

use anyhow::{bail, Context};
use thiserror::Error;

use labkit::logln;

fn main() {
    demo_thiserror();
    demo_anyhow();
}

// ============ 库的姿势：thiserror ============

/// 一个精确、可 match 的库错误。三个属性替你写光模板。
#[derive(Debug, Error)]
enum ConfigError {
    #[error("配置文件 {path} 不存在")] // 这行就是 Display
    NotFound { path: String },

    #[error("解析失败: {0}")]
    Parse(String),

    #[error("IO 错误")]
    Io(#[from] std::io::Error), // #[from] 自动生成 From<io::Error>
}

/// 尝试读一个配置：文件不存在时返回精确的 NotFound。
fn load_config(path: &str) -> Result<String, ConfigError> {
    if path.is_empty() {
        return Err(ConfigError::Parse("路径为空".into()));
    }
    // ? 会把 io::Error 通过 #[from] 自动转成 ConfigError::Io
    let content = std::fs::read_to_string(path).map_err(|_| ConfigError::NotFound { path: path.to_string() })?;
    Ok(content)
}

fn demo_thiserror() {
    logln!("--- thiserror：库的精确错误 ---");
    match load_config("绝对不存在.toml") {
        Ok(_) => logln!("  居然读到了"),
        // 可以针对某种变体特殊处理
        Err(ConfigError::NotFound { path }) => logln!("  文件不存在，将创建默认：{path}"),
        Err(e) => logln!("  其它错误：{e}"),
    }
}

// ============ 应用的姿势：anyhow ============

/// 应用层：用 anyhow 一路 ? + context，最后打印完整因果链。
fn run_app() -> anyhow::Result<i64> {
    let raw = std::fs::read_to_string("绝对不存在.toml").context("读取配置文件失败")?; // 叠一层上下文
    let n: i64 = raw.trim().parse().context("配置内容不是数字")?;
    if n < 0 {
        bail!("端口不能为负：{n}"); // = return Err(anyhow!(...))
    }
    Ok(n)
}

fn demo_anyhow() {
    logln!("--- anyhow：应用的错误盒子 + 上下文链 ---");
    match run_app().context("应用启动失败") {
        Ok(n) => logln!("  启动成功，端口 {n}"),
        Err(e) => {
            // {:#} 打印完整因果链（Caused by: ...）
            logln!("  失败，完整因果链：");
            for (i, cause) in e.chain().enumerate() {
                logln!("    {i}: {cause}");
            }
        }
    }
}
