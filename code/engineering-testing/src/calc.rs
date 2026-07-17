//! 纯函数模块：最好测的一类代码（无状态、无 IO、输入定输出）。

/// 解析 "key=value" 形式的字符串。
///
/// 下面这段带 ``` 的示例是**文档测试（doc-test）**：`cargo test` 会把它抠出来
/// 编译执行——文档里的例子永远不会过时（对照 godoc 的 Example 函数，但 Go 的
/// Example 只比对输出，Rust 直接跑断言）。
///
/// ```
/// let (k, v) = engineering_testing::calc::parse_kv("name=tokio").unwrap();
/// assert_eq!(k, "name");
/// assert_eq!(v, "tokio");
///
/// assert!(engineering_testing::calc::parse_kv("没有等号").is_err());
/// ```
pub fn parse_kv(s: &str) -> Result<(String, String), String> {
    match s.split_once('=') {
        Some((k, v)) if !k.trim().is_empty() => Ok((k.trim().to_string(), v.trim().to_string())),
        Some(_) => Err(format!("key 为空: {s:?}")),
        None => Err(format!("缺少等号: {s:?}")),
    }
}

/// 求平均值；空切片返回 None（把"没有数据"显式化，00 课 §5.1 的哲学）。
pub fn mean(xs: &[f64]) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    Some(xs.iter().sum::<f64>() / xs.len() as f64)
}
