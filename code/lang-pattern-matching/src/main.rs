//! 语言地基 · 模式匹配与枚举 —— 可运行示例
//!
//! 配套文档：docs/lang/pattern-matching.md
//! 运行：cargo run -p lang-pattern-matching（先 cd code）

use labkit::logln;

fn main() {
    demo_exhaustive();
    demo_destructure();
    demo_guard();
    demo_range_or();
    demo_at_bind();
    demo_if_let();
    demo_while_let();
    demo_let_else();
    demo_ignore();
    demo_matches();
}

// ---- 穷尽性 ----

enum Status {
    Active,
    Paused,
    Deleted,
}

fn describe(s: &Status) -> &'static str {
    // 少写任何一个变体都会编译报错 E0004
    match s {
        Status::Active => "运行中",
        Status::Paused => "已暂停",
        Status::Deleted => "已删除",
    }
}

fn demo_exhaustive() {
    logln!("--- 穷尽性 ---");
    for s in [Status::Active, Status::Paused, Status::Deleted] {
        logln!("  {}", describe(&s));
    }
}

// ---- 解构 ----

enum Msg {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
    Color(u8, u8, u8),
}

fn handle(m: Msg) {
    match m {
        Msg::Quit => logln!("  退出"),
        Msg::Move { x, y } => logln!("  移动到 ({x}, {y})"), // 拆出 x、y
        Msg::Write(text) => logln!("  文本：{text}"),        // 拆出 text
        Msg::Color(r, g, b) => logln!("  颜色 #{r:02x}{g:02x}{b:02x}"),
    }
}

fn demo_destructure() {
    logln!("--- 解构 ---");
    handle(Msg::Move { x: 3, y: 4 });
    handle(Msg::Write("你好".into()));
    handle(Msg::Color(255, 128, 0));
    handle(Msg::Quit);

    // 元组解构
    let point = (0, 7);
    match point {
        (0, 0) => logln!("  原点"),
        (0, y) => logln!("  在 Y 轴上，y={y}"),
        (x, 0) => logln!("  在 X 轴上，x={x}"),
        (x, y) => logln!("  其它点 ({x}, {y})"),
    }
}

// ---- 守卫 ----

fn demo_guard() {
    logln!("--- 守卫 ---");
    for score in [Some(95), Some(70), Some(40), None] {
        let msg = match score {
            Some(s) if s >= 90 => format!("优秀 {s}"),
            Some(s) if s >= 60 => format!("及格 {s}"),
            Some(s) => format!("不及格 {s}"),
            None => "缺考".to_string(),
        };
        logln!("  {msg}");
    }
}

// ---- 范围 / 或模式 ----

fn demo_range_or() {
    logln!("--- 范围与或模式 ---");
    for c in ['k', 'Z', '7', '@'] {
        let kind = match c {
            'a'..='z' => "小写字母",
            'A'..='Z' => "大写字母",
            '0'..='9' => "数字",
            _ => "其它字符",
        };
        logln!("  {c} -> {kind}");
    }
    for day in [3, 6] {
        let kind = match day {
            1 | 2 | 3 | 4 | 5 => "工作日",
            6 | 7 => "周末",
            _ => "非法",
        };
        logln!("  第 {day} 天 -> {kind}");
    }
}

// ---- @ 绑定 ----

fn demo_at_bind() {
    logln!("--- @ 绑定 ---");
    for n in [5, 42, 500] {
        match n {
            x @ 1..=9 => logln!("  个位数：{x}"),
            x @ 10..=99 => logln!("  两位数：{x}"),
            x => logln!("  其它：{x}"),
        }
    }
}

// ---- if let ----

fn demo_if_let() {
    logln!("--- if let ---");
    let config: Option<&str> = Some("debug");
    if let Some(mode) = config {
        logln!("  模式：{mode}");
    } else {
        logln!("  用默认模式");
    }
}

// ---- while let ----

fn demo_while_let() {
    logln!("--- while let ---");
    let mut stack = vec![1, 2, 3];
    while let Some(top) = stack.pop() {
        logln!("  弹出 {top}"); // 依次 3、2、1，pop 返回 None 时停
    }
}

// ---- let else ----

fn parse_port(s: &str) -> Result<u16, String> {
    let Ok(port) = s.parse::<u16>() else {
        return Err(format!("不是合法端口：{s}"));
    };
    Ok(port) // 走到这，port 一定有值
}

fn demo_let_else() {
    logln!("--- let else ---");
    logln!("  {:?}", parse_port("8080"));
    logln!("  {:?}", parse_port("abc"));
}

// ---- 忽略 _ 和 .. ----

fn demo_ignore() {
    logln!("--- 忽略 _ 和 .. ---");
    let tuple = (1, 2, 3, 4, 5);
    let (first, .., last) = tuple; // 只要头尾
    logln!("  {first} ... {last}");

    let point = (10, 20, 30);
    let (x, ..) = point; // 只要第一个
    logln!("  x = {x}");
}

// ---- matches! ----

fn demo_matches() {
    logln!("--- matches! ---");
    let x = Some(5);
    let big = matches!(x, Some(n) if n > 3); // 要一个 bool
    logln!("  Some(5) 是不是 >3 的 Some？{big}");
}
