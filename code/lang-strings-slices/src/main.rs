//! 语言地基 · 字符串、数组与切片 —— 可运行示例
//!
//! 配套文档：docs/lang/strings-slices.md
//! 运行：cargo run -p lang-strings-slices（先 cd code）

use std::path::PathBuf;

use labkit::logln;

fn main() {
    demo_string_vs_str();
    demo_param_prefers_str();
    demo_utf8_rules();
    demo_string_ops();
    demo_split_chain();
    demo_array();
    demo_slice_views();
    demo_pathbuf();
}

/// String 拥有、&str 是视图；互转与函数参数选型。
fn demo_string_vs_str() {
    logln!("--- String 与 &str ---");

    let owned = String::from("拥有");
    let view: &str = &owned; // String 可自动降级为 &str
    let literal: &str = "字面量"; // 字面量类型就是 &'static str
    logln!("owned={owned}, view={view}, literal={literal}");

    let again = view.to_string(); // 视图 → 拥有
    logln!("to_string 后 again={again}");
}

/// 函数参数默认收 &str：字面量和 String 都能传。
fn demo_param_prefers_str() {
    logln!("--- 参数用 &str ---");

    greet("Gopher"); // 字面量
    greet(&String::from("Rust")); // String 借用
}

fn greet(name: &str) {
    logln!("  你好，{name}！");
}

/// len 是字节数；不许下标取字符；切片必须落在字符边界。
fn demo_utf8_rules() {
    logln!("--- UTF-8 硬规矩 ---");

    let s = "你好a";
    logln!("  len()（字节）= {}", s.len()); // 7 = 3+3+1
    logln!("  chars().count()（字符）= {}", s.chars().count()); // 3
    logln!("  chars().nth(1) = {:?}", s.chars().nth(1)); // Some('好')
    logln!("  &s[0..3]（落在边界）= {}", &s[0..3]); // "你"
    // &s[0..4] → panic：字节 4 不是字符边界（请在文档实验里亲手撞）
}

/// 常见字符串操作对照 Go strings 包。
fn demo_string_ops() {
    logln!("--- 字符串操作 ---");

    let s = "  Hello, Rust!  ";
    logln!("  contains Rust? {}", s.contains("Rust"));
    logln!("  starts_with Hello? {}", s.trim().starts_with("Hello"));
    logln!("  trim = [{}]", s.trim());
    logln!("  upper = {}", "hi".to_uppercase());
    logln!("  replace = {}", "a-b-a".replace('a', "x"));
    logln!("  parse = {:?}", "42".parse::<i64>());
    logln!("  format! = {}", format!("n={}", 7));

    let parts = ["a", "b", "c"];
    logln!("  join = {}", parts.join("-"));
}

/// split 返回迭代器，可直接接 map/filter/collect。
fn demo_split_chain() {
    logln!("--- split 接链 ---");

    let nums: Vec<i64> = "1, 2, , 3"
        .split(',')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| p.parse().unwrap())
        .collect();
    logln!("  解析结果 = {nums:?}");
}

/// 定长数组：长度写进类型；重复初始化；可整体 Copy（元素是 Copy 时）。
fn demo_array() {
    logln!("--- 定长数组 ---");

    let nums: [i32; 3] = [10, 20, 30];
    let zeros = [0i32; 4];
    logln!("  nums={nums:?}, zeros={zeros:?}");
    logln!("  nums[1]={}, get(99)={:?}", nums[1], nums.get(99));
}

/// &[T] / &mut [T]：借用视图；与 Vec 的关系。
fn demo_slice_views() {
    logln!("--- 切片视图 ---");

    let mut v = vec![1, 2, 3, 4, 5];
    let mid: &[i32] = &v[1..4];
    logln!("  &v[1..4] = {mid:?}");

    let (left, right) = v.split_at(2);
    logln!("  split_at(2): left={left:?}, right={right:?}");

    // 可变切片：改一段
    let tail: &mut [i32] = &mut v[3..];
    tail[0] = 99;
    logln!("  改尾部后 v = {v:?}");

    // 数组也能切出切片
    let arr = [7, 8, 9];
    let s: &[i32] = &arr[1..];
    logln!("  数组切片 &arr[1..] = {s:?}");
}

/// 路径用 PathBuf，别手拼 String。
fn demo_pathbuf() {
    logln!("--- PathBuf ---");

    let mut p = PathBuf::from("/data");
    p.push("logs");
    p.push("app.log");
    logln!("  path = {}", p.display());
    logln!("  extension = {:?}", p.extension());
}
