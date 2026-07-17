//! 语言地基 · 泛型与 trait bound —— 可运行示例
//!
//! 配套文档：docs/lang/generics.md
//! 运行：cargo run -p lang-generics（先 cd code）

use std::fmt::Display;

use labkit::logln;

fn main() {
    demo_generic_fn();
    demo_generic_struct();
    demo_impl_trait_return();
    demo_blanket();
    demo_dyn();
}

// ---- 泛型函数 + trait bound ----

/// 对任何"能比大小"的 T 求最大值。约束 T: PartialOrd。
fn largest<T: PartialOrd>(list: &[T]) -> &T {
    let mut max = &list[0];
    for x in list {
        if x > max {
            max = x;
        }
    }
    max
}

fn demo_generic_fn() {
    logln!("--- 泛型函数 ---");
    // 编译器为 i32 和 &str 各单态化一份 largest
    logln!("  最大的数 = {}", largest(&[3, 7, 2, 9, 4]));
    logln!("  最大的词 = {}", largest(&["pear", "apple", "kiwi"]));
}

// ---- 泛型结构体 + where ----

struct Pair<T> {
    a: T,
    b: T,
}

impl<T> Pair<T> {
    fn new(a: T, b: T) -> Self {
        Pair { a, b }
    }
}

/// where 子句：约束多时把它挪到签名后面更清爽。
fn show_pair<T>(p: &Pair<T>)
where
    T: Display,
{
    logln!("  ({}, {})", p.a, p.b);
}

fn demo_generic_struct() {
    logln!("--- 泛型结构体 + where ---");
    let p = Pair::new(1, 2);
    show_pair(&p);
    let q = Pair::new("左", "右");
    show_pair(&q);
}

// ---- impl Trait 返回位置 ----

/// 返回一个迭代器，但不写它那一长串具体类型，用 impl Iterator 带过。
fn evens(max: u32) -> impl Iterator<Item = u32> {
    (0..max).filter(|n| n % 2 == 0)
}

fn demo_impl_trait_return() {
    logln!("--- impl Trait 返回迭代器 ---");
    let list: Vec<u32> = evens(10).collect();
    logln!("  0..10 的偶数 = {list:?}");
}

// ---- blanket impl ----

trait Summary {
    fn summarize(&self) -> String;
}

/// 给"所有实现了 Display 的类型"一次性实现 Summary。
impl<T: Display> Summary for T {
    fn summarize(&self) -> String {
        format!("摘要：{self}")
    }
}

fn demo_blanket() {
    logln!("--- blanket impl（一次给一片）---");
    // i32 和 String 都实现了 Display，于是自动都有了 .summarize()
    logln!("  {}", 42.summarize());
    logln!("  {}", "hello".to_string().summarize());
}

// ---- dyn：异构集合 ----

trait Draw {
    fn draw(&self) -> &'static str;
}

struct Circle;
struct Square;

impl Draw for Circle {
    fn draw(&self) -> &'static str {
        "○"
    }
}
impl Draw for Square {
    fn draw(&self) -> &'static str {
        "□"
    }
}

fn demo_dyn() {
    logln!("--- dyn：一个 Vec 装多种类型 ---");
    // Vec<Box<dyn Draw>>：装"任何实现了 Draw 的东西"
    let shapes: Vec<Box<dyn Draw>> = vec![Box::new(Circle), Box::new(Square)];
    for s in &shapes {
        logln!("  {}", s.draw());
    }
}
