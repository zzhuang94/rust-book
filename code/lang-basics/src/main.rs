//! 语言地基 · 基础类型 —— 可运行示例
//!
//! 配套文档：docs/lang/basics.md
//! 运行：cargo run -p lang-basics（先 cd code）
//!
//! 本文件只放能编过、能跑的正面示例；文档里故意编不过的反例请对照阅读。

use std::mem::size_of;

use labkit::logln;

fn main() {
    demo_integers();
    demo_float_bool_char();
    demo_inference_and_annotate();
    demo_no_implicit_cast();
    demo_tuple();
    demo_array();
    demo_unit_and_underscore();
    demo_shadowing();
}

/// 整数：位数写进类型名；字面量可用下划线分隔、可用后缀定型。
fn demo_integers() {
    logln!("--- 整数 ---");

    let a: i32 = 42; // 有符号 32 位，日常默认选型
    let b: u64 = 1_000_000; // 无符号 64 位；下划线只为好看，不影响值
    let c = 255u8; // 后缀定型：这是 u8，不是默认的 i32
    let hex = 0xff_u16; // 十六进制 + 后缀
    let bin = 0b1010_u8; // 二进制
    logln!("i32={a}, u64={b}, u8={c}, hex={hex}, bin={bin}");

    // 对照 Go：int 宽度随平台；Rust 的 i32/i64 宽度固定，跨平台行为一致。
}

/// 浮点、布尔、字符：char 是 Unicode 标量，占 4 字节，不是单字节。
fn demo_float_bool_char() {
    logln!("--- 浮点 / bool / char ---");

    let pi: f64 = 3.14159; // 浮点默认推断成 f64（不是 f32）
    let ok = true; // bool：只有 true / false，没有 0/1 冒充
    let ch = '🦀'; // char：一个 Unicode 标量值（可以是 emoji）
    let ascii = 'A';
    logln!("pi={pi}, ok={ok}, ch={ch}, ascii={ascii}, char 大小={} 字节", size_of::<char>());
}

/// 类型推断很聪明，但「该写标注时就写」——尤其是空集合和数值边界。
fn demo_inference_and_annotate() {
    logln!("--- 推断与标注 ---");

    let x = 10; // 推断为 i32（整数默认）
    let y = 2.5; // 推断为 f64（浮点默认）
    logln!("推断：x={x} (i32), y={y} (f64)");

    // 空 Vec 看不出元素类型，必须标注（或后面用一次让编译器推断）
    let mut ids: Vec<i64> = Vec::new();
    ids.push(7);
    logln!("标注后的 Vec<i64> = {ids:?}");

    // 左边写类型 = 帮编译器（也帮读者）定型，对照 Go 的 var x int64 = 1
    let n: i64 = 1;
    logln!("显式标注 i64：{n}");
}

/// Rust 数值**从不**隐式转换：i32 不能直接加 i64，必须 as 或 Into。
fn demo_no_implicit_cast() {
    logln!("--- 显式转换 ---");

    let a: i32 = 10;
    let b: i64 = 20;
    // let sum = a + b; // ❌ 编译错误：mismatched types
    let sum = a as i64 + b; // as：按位宽转换（截断/符号扩展按规则来）
    logln!("a as i64 + b = {sum}");

    let big: i32 = 300;
    let truncated = big as u8; // 300 塞进 u8 会截断——合法但危险，生产慎用
    logln!("300 as u8 = {truncated}（截断演示，真实代码优先用 TryFrom）");

    // From / Into：更安全、更有语义的转换（能转才实现）
    let n: i32 = 5;
    let wide: i64 = n.into(); // i32 → i64 一定安全，所以有 Into
    logln!("i32.into() → i64 = {wide}");
}

/// 元组：把几个值临时捆成一组；用 .0/.1 或解构拆开。
fn demo_tuple() {
    logln!("--- 元组 ---");

    let pair: (i32, &str) = (1, "hello"); // 类型各异也能捆
    logln!("pair.0={}, pair.1={}", pair.0, pair.1);

    let (id, name) = pair; // 解构：一次拆到两个变量（对照 Go 的多返回值拆包）
    logln!("解构后 id={id}, name={name}");

    // 单元素元组必须写逗号，否则就是普通括号表达式
    let one = (42,);
    logln!("单元素元组 = {one:?}");
}

/// 定长数组：长度写进类型；索引越界直接 panic（debug/release 都查）。
fn demo_array() {
    logln!("--- 定长数组 ---");

    let nums: [i32; 3] = [10, 20, 30]; // [元素类型; 长度]
    let zeros = [0i32; 5]; // 5 个 0 —— 重复初始化语法
    logln!("nums={nums:?}, len={}, zeros={zeros:?}", nums.len());

    let first = nums[0];
    logln!("nums[0]={first}");

    // 数组可以整体拷贝（元素是 Copy 时）；想「一段视图」用切片，见字符串/切片章
    let copy = nums;
    logln!("数组按值拷贝后 copy={copy:?}，原 nums 仍可用={nums:?}");
}

/// unit 类型 ()：表示「没有有意义的值」；_ 丢弃不想用的值。
fn demo_unit_and_underscore() {
    logln!("--- unit 与 _ ---");

    let u: () = (); // unit 值只有一个，就长这样
    logln!("unit 调试打印 = {u:?}");

    let pair = (1, 2, 3);
    let (x, _, z) = pair; // _ 表示「这个位置我不要」
    logln!("只要两头：x={x}, z={z}");
}

/// 遮蔽：同名 let 重新绑定，旧绑定被遮住——类型都可以变。Go 没有这招。
fn demo_shadowing() {
    logln!("--- 遮蔽 shadowing ---");

    let x = 5;
    let x = x + 1; // 合法：新的 x 遮住旧的（现在是 6）
    logln!("同类型遮蔽后 x = {x}");
    let x = "现在是字符串"; // 连类型都能换——因为这是新变量，不是 mut 修改
    logln!("换类型遮蔽后 x = {x}");
}
