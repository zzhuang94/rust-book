//! 语言地基 · 集合：Vec 与 HashMap —— 可运行示例
//!
//! 配套文档：docs/lang/collections.md
//! 运行：cargo run -p lang-collections（先 cd code）

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use labkit::logln;

fn main() {
    demo_vec_basics();
    demo_vec_safe_index();
    demo_hashmap_no_zero();
    demo_entry_wordcount();
    demo_entry_group();
    demo_hashset();
    demo_btreemap();
    demo_vecdeque();
}

/// Vec ≈ Go slice：push/pop/sort/retain 等日常操作。
fn demo_vec_basics() {
    logln!("--- Vec 常用操作 ---");

    let mut v = vec![3, 1, 2];
    v.push(4);
    logln!("  push 后 = {v:?}");

    let last = v.pop(); // Option
    logln!("  pop = {:?}, 剩余 = {v:?}", last);

    v.sort();
    logln!("  sort 后 = {v:?}");

    v.retain(|x| *x > 1);
    logln!("  retain(>1) = {v:?}");
    logln!("  contains(2)? {}，first={:?}", v.contains(&2), v.first());
}

/// 业务代码优先 get；下标越界会 panic。
fn demo_vec_safe_index() {
    logln!("--- 安全取下标 ---");

    let v = vec!["甲", "乙"];
    match v.get(1) {
        Some(x) => logln!("  get(1) = {x}"),
        None => logln!("  没有"),
    }
    logln!("  get(99) = {:?}", v.get(99));
    // v[99] → panic，服务里别这么干
}

/// HashMap：get 返回 Option，没有「零值冒充有值」。
fn demo_hashmap_no_zero() {
    logln!("--- HashMap 没有零值 ---");

    let mut m: HashMap<String, i64> = HashMap::new();
    m.insert("a".into(), 1);

    match m.get("b") {
        Some(v) => logln!("  有 b: {v}"),
        None => logln!("  没有 b"),
    }
    let v = m.get("b").copied().unwrap_or(0);
    logln!("  亲口说「没有当 0」→ {v}");

    // 遍历：顺序不稳定（对照 Go range map）
    for (k, v) in &m {
        logln!("  遍历 {k} = {v}");
    }
}

/// entry API 词频统计名场面。
fn demo_entry_wordcount() {
    logln!("--- entry 词频 ---");

    let text = "苹果 香蕉 苹果 橙子 香蕉 苹果";
    let mut freq: HashMap<&str, i64> = HashMap::new();
    for w in text.split_whitespace() {
        *freq.entry(w).or_insert(0) += 1;
    }

    let mut pairs: Vec<_> = freq.into_iter().collect();
    pairs.sort_by_key(|(_, cnt)| std::cmp::Reverse(*cnt));
    for (w, c) in pairs {
        logln!("  {w}: {c}");
    }
}

/// entry + or_insert_with：分组收集。
fn demo_entry_group() {
    logln!("--- entry 分组 ---");

    let items = [("水果", "苹果"), ("水果", "香蕉"), ("蔬菜", "青菜")];
    let mut groups: HashMap<&str, Vec<&str>> = HashMap::new();
    for (cat, name) in items {
        groups.entry(cat).or_insert_with(Vec::new).push(name);
    }
    for (cat, names) in &groups {
        logln!("  {cat} → {names:?}");
    }
}

/// HashSet：真正的集合（Go 常用 map[T]bool 模拟）。
fn demo_hashset() {
    logln!("--- HashSet ---");

    let mut set = HashSet::from(["a", "b", "a"]); // 重复自动去
    set.insert("c");
    logln!("  set = {set:?}, 含 b? {}", set.contains("b"));
}

/// BTreeMap：按 key 有序（Go 要自己收集再 sort）。
fn demo_btreemap() {
    logln!("--- BTreeMap 有序 ---");

    let mut m = BTreeMap::new();
    m.insert("c", 3);
    m.insert("a", 1);
    m.insert("b", 2);
    for (k, v) in &m {
        logln!("  {k} = {v}"); // a, b, c 顺序
    }
}

/// VecDeque：两端都能 O(1) push/pop。
fn demo_vecdeque() {
    logln!("--- VecDeque ---");

    let mut q = VecDeque::from([2, 3]);
    q.push_front(1);
    q.push_back(4);
    logln!("  队列 = {q:?}");
    logln!("  pop_front = {:?}, 剩余 = {q:?}", q.pop_front());
}
