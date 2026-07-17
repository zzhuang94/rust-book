//! 语言地基 · 流程控制 —— 可运行示例
//!
//! 配套文档：docs/lang/control-flow.md
//! 运行：cargo run -p lang-control-flow（先 cd code）

use labkit::logln;

fn main() {
    demo_if_expression();
    demo_else_if();
    demo_loop_break_value();
    demo_while();
    demo_for_range();
    demo_for_collection();
    demo_continue();
    demo_loop_label();
}

/// if 是表达式：两个分支必须类型一致，整个 if 可以有返回值。
fn demo_if_expression() {
    logln!("--- if 是表达式 ---");

    let n = 7;
    // 对照 Go：n % 2 == 0 ? "偶" : "奇" —— Go 没有三元运算，Rust 用 if 表达式顶上
    let parity = if n % 2 == 0 { "偶" } else { "奇" };
    logln!("{n} 是{parity}数");

    // 分支里最后是「不带分号的表达式」时，它就是该分支的值
    let abs = if n >= 0 { n } else { -n };
    logln!("绝对值 = {abs}");
}

/// else if 链：和 Go 一样读；条件必须是 bool，不能写 if n。
fn demo_else_if() {
    logln!("--- else if ---");

    let score = 85;
    let grade = if score >= 90 {
        "A"
    } else if score >= 80 {
        "B"
    } else if score >= 60 {
        "C"
    } else {
        "D"
    };
    logln!("分数 {score} → 等级 {grade}");
}

/// loop 无限循环；break 可以带走一个值（Go 的 break 做不到）。
fn demo_loop_break_value() {
    logln!("--- loop + break 带值 ---");

    let mut i = 0;
    // break 后面的表达式 = 整个 loop 表达式的值
    let found = loop {
        i += 1;
        if i == 3 {
            break i * 10; // 带走 30
        }
    };
    logln!("loop 找到了 {found}");
}

/// while：条件为真就转；没有 while-true 惯用时更常直接写 loop。
fn demo_while() {
    logln!("--- while ---");

    let mut n = 3;
    while n > 0 {
        logln!("  倒计时 {n}");
        n -= 1;
    }
    logln!("发射！");
}

/// for + 区间：0..n 半开，0..=n 闭合；这是最常见的「数数」写法。
fn demo_for_range() {
    logln!("--- for 区间 ---");

    // 0..3 → 0,1,2（不含 3），对照 Go: for i := 0; i < 3; i++
    for i in 0..3 {
        logln!("  半开 0..3 → i={i}");
    }

    // 0..=3 → 0,1,2,3（含 3）
    for i in 0..=3 {
        logln!("  闭合 0..=3 → i={i}");
    }

    // 带步长：用 (0..10).step_by(2)，没有 Go 那种 for i := 0; i < 10; i += 2 语法糖
    for i in (0..6).step_by(2) {
        logln!("  step_by(2) → i={i}");
    }
}

/// for 遍历集合：默认 for x in v 会「吃掉」v（move）；只看请用 &v。
fn demo_for_collection() {
    logln!("--- for 集合 ---");

    let v = vec!["甲", "乙", "丙"];

    // 借来看：不夺走所有权，循环后 v 还能用（所有权章会细讲）
    for item in &v {
        logln!("  借用元素：{item}");
    }
    logln!("借完之后 v 仍在，长度 = {}", v.len());

    // 要下标：iter().enumerate()，对照 Go 的 for i, x := range v
    for (i, item) in v.iter().enumerate() {
        logln!("  下标 {i} → {item}");
    }
}

/// continue：跳过本轮剩余部分，和 Go 同名同义。
fn demo_continue() {
    logln!("--- continue ---");

    for i in 0..5 {
        if i % 2 == 0 {
            continue; // 偶数轮直接下一轮
        }
        logln!("  奇数 i={i}");
    }
}

/// 循环标签：多层循环时 break/continue 可以指定跳到哪一层（Go 也有 label）。
fn demo_loop_label() {
    logln!("--- 循环标签 ---");

    let mut hits = 0;
    'outer: for x in 1..=3 {
        for y in 1..=3 {
            hits += 1;
            if x == 2 && y == 2 {
                logln!("  在 ({x},{y}) 处跳出外层");
                break 'outer; // 不只破内层，连 outer 一起停
            }
        }
    }
    logln!("一共转了 {hits} 次内层（若无标签会更多）");
}
