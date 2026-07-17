//! 语言地基 · 迭代器 —— 可运行示例
//!
//! 配套文档：docs/lang/iterators.md
//! 运行：cargo run -p lang-iterators（先 cd code）

use std::collections::HashMap;

use labkit::logln;

fn main() {
    demo_three_entries();
    demo_for_in_moves();
    demo_lazy_chain();
    demo_adapters_gallery();
    demo_collect_targets();
    demo_collect_results();
    demo_when_for();
}

/// 三种入口：iter / iter_mut / into_iter —— 对应三种借用。
fn demo_three_entries() {
    logln!("--- 三种入口 ---");

    let mut v = vec![1, 2, 3];

    let sum: i32 = v.iter().sum(); // &i32
    logln!("  iter sum={sum}，v 仍在 {v:?}");

    for x in v.iter_mut() {
        // &mut i32
        *x *= 10;
    }
    logln!("  iter_mut 后 {v:?}");

    let owned: Vec<i32> = v.into_iter().map(|x| x + 1).collect();
    logln!("  into_iter 加工后 {owned:?}（原 v 已作废）");
}

/// for x in v 默认 into_iter；只看请写 &v。
fn demo_for_in_moves() {
    logln!("--- for in 与 move ---");

    let v = vec!["甲", "乙", "丙"];
    for item in &v {
        logln!("  借用：{item}");
    }
    logln!("  &v 之后还能用，len={}", v.len());

    for (i, item) in v.iter().enumerate() {
        logln!("  enumerate {i} → {item}");
    }
}

/// 惰性：搭流水线不干活，collect/sum 等末端才执行。
fn demo_lazy_chain() {
    logln!("--- 惰性链 ---");

    let ages = [15, 22, 30, 17, 40, 25];
    let result: Vec<i32> = ages
        .iter()
        .filter(|&&a| a >= 18)
        .map(|&a| a * 2)
        .take(3)
        .collect();
    logln!("  成年×2 取前3 = {result:?}");
}

/// 常用适配器速览。
fn demo_adapters_gallery() {
    logln!("--- 适配器速览 ---");

    let nums = [1, 2, 3, 4, 5, 6];

    let evens: Vec<_> = nums.iter().copied().filter(|n| n % 2 == 0).collect();
    logln!("  filter 偶数 = {evens:?}");

    let zipped: Vec<_> = ["a", "b"].iter().zip([1, 2]).collect();
    logln!("  zip = {zipped:?}");

    let any_big = nums.iter().any(|&n| n > 5);
    let all_pos = nums.iter().all(|&n| n > 0);
    logln!("  any>5? {any_big}, all>0? {all_pos}");

    let found = nums.iter().find(|&&n| n > 3);
    logln!("  find >3 = {found:?}");

    let folded = nums.iter().fold(0, |acc, n| acc + n);
    logln!("  fold 求和 = {folded}");
}

/// collect：左边类型决定装成什么。
fn demo_collect_targets() {
    logln!("--- collect 目标类型 ---");

    let as_vec: Vec<i64> = (1..=5).collect();
    logln!("  → Vec = {as_vec:?}");

    let as_string: String = ['h', 'i', '!'].into_iter().collect();
    logln!("  → String = {as_string}");

    let pairs = [(1u32, "a"), (2, "b")];
    let as_map: HashMap<u32, &str> = pairs.into_iter().collect();
    logln!("  → HashMap = {as_map:?}");
}

/// Vec<Result> collect 成 Result<Vec>：一个失败则整体失败。
fn demo_collect_results() {
    logln!("--- collect Result ---");

    let ok: Result<Vec<i64>, _> = ["1", "2", "3"]
        .into_iter()
        .map(|s| s.parse::<i64>())
        .collect();
    let bad: Result<Vec<i64>, _> = ["1", "x", "3"]
        .into_iter()
        .map(|s| s.parse::<i64>())
        .collect();
    logln!("  全合法 → {ok:?}");
    logln!(
        "  含非法 → {}",
        if bad.is_err() {
            "Err（整体失败）"
        } else {
            "Ok"
        }
    );
}

/// 有副作用、复杂 break 时，for 往往更清晰。
fn demo_when_for() {
    logln!("--- 何时用 for ---");

    let mut total = 0;
    let mut hits = 0;
    for n in 1..=10 {
        if n % 2 == 0 {
            continue;
        }
        total += n;
        hits += 1;
        if total > 15 {
            logln!("  累加奇数超过 15，提前停（hits={hits}, total={total}）");
            break;
        }
    }
}
