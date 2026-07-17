//! 集成测试：tests/ 目录下每个 .rs 文件被编译成一个独立测试程序，
//! 以"外部使用者"的身份 use 被测 crate（只能碰 pub 的东西）。
//! 跑法：cargo test -p engineering-testing

use engineering_testing::calc::{mean, parse_kv};

#[test] // 普通同步测试：一个被 #[test] 标记的函数，panic 即失败
fn parse_kv_正常路径() {
    let (k, v) = parse_kv("name = tokio ").unwrap();
    assert_eq!(k, "name"); // 失败时会打印 left/right 两边的值
    assert_eq!(v, "tokio");
}

#[test]
fn parse_kv_错误路径() {
    // 断言"是 Err 且错误信息包含关键字"——错误路径也是一等公民
    let err = parse_kv("没有等号").unwrap_err();
    assert!(err.contains("缺少等号"), "实际错误: {err}");

    // matches!：断言值符合某个模式（00e 的 match 用在断言里）
    assert!(matches!(parse_kv("=value"), Err(_)));
}

#[test]
fn mean_基本与空输入() {
    assert_eq!(mean(&[1.0, 2.0, 3.0]), Some(2.0));
    assert_eq!(mean(&[]), None); // "没有数据"是显式的 None，不是 NaN
}
