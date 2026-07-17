//! 语言地基 · 所有权与借用 —— 可运行示例
//!
//! 配套文档：docs/lang/ownership.md
//! 运行：cargo run -p lang-ownership（先 cd code）
//!
//! 只含能编过的正面示例；文档里故意编不过的反例请对照阅读、亲手撞报错。
//! 生命周期专章示例见：cargo run -p lang-lifetimes

use labkit::logln;

fn main() {
    demo_move_and_clone();
    demo_move_into_fn();
    demo_return_ownership();
    demo_copy();
    demo_drop_order();
    demo_borrow();
    demo_many_readers_or_one_writer();
    demo_nll();
    demo_scope_ends_borrow();
    demo_slice_view();
}

/// move：赋值默认交接所有权；两个都要用就 .clone()。
fn demo_move_and_clone() {
    logln!("--- move 与 clone ---");

    let a = String::from("你好");
    let b = a; // 所有权从 a 交给 b；此后不能再用 a
    logln!("move 之后只有 b 能用: {b}");

    let c = String::from("苹果");
    let d = c.clone(); // 深拷贝堆数据，c、d 各有一份
    logln!("clone 之后都能用: c={c}, d={d}");
}

/// 传参也是 move：函数吃掉 String 后，调用方变量作废。
fn demo_move_into_fn() {
    logln!("--- 传参 move ---");

    let fruit = String::from("香蕉");
    eat(fruit);
    // logln!("{fruit}"); // ❌ E0382：已 move
    logln!("fruit 已交给 eat，调用方不能再用");
}

fn eat(s: String) {
    logln!("  吃掉 {s}（函数结束时 s 被 drop）");
}

/// 返回值把所有权交还给调用方——这是「函数造数据」的常规写法。
fn demo_return_ownership() {
    logln!("--- 返回所有权 ---");

    let s = make_greeting("Gopher");
    logln!("拿到所有权：{s}");
}

fn make_greeting(name: &str) -> String {
    format!("你好，{name}！") // 新 String 的所有权返回给调用方
}

/// Copy：纯栈小类型赋值即复制，旧变量不作废。
fn demo_copy() {
    logln!("--- Copy ---");

    let x = 5;
    let y = x; // i32: Copy，不是 move
    logln!("Copy 之后 x={x}, y={y}");

    let p = (1i32, true); // 字段都是 Copy → 元组也是 Copy
    let q = p;
    logln!("Copy 元组 p={p:?}, q={q:?}");
}

/// Drop / RAII：离开作用域自动收尾；后创建的先释放。
fn demo_drop_order() {
    logln!("--- Drop 顺序（后进先出）---");

    let _a = Guard("外层");
    {
        let _b = Guard("内层");
    } // ← 内层在这里 drop
    logln!("内层块已结束");
} // ← 外层在这里 drop

struct Guard(&'static str);

impl Drop for Guard {
    fn drop(&mut self) {
        logln!("  {} 被释放", self.0);
    }
}

/// 借用：& 只读、&mut 可写；都不夺走所有权。
fn demo_borrow() {
    logln!("--- 借用 ---");

    let mut s = String::from("你好");
    let n = char_count(&s);
    logln!("只读借用：字符数={n}，s 仍在={s}");

    append_bang(&mut s);
    logln!("可变借用改完：s={s}");
}

fn char_count(s: &String) -> usize {
    s.chars().count()
}

fn append_bang(s: &mut String) {
    s.push('！');
}

/// 铁律：多个只读 XOR 一个可写。读用完（NLL）后才能再可变借。
fn demo_many_readers_or_one_writer() {
    logln!("--- 多读 XOR 一写 ---");

    let mut v = vec![1, 2, 3];
    let r1 = &v;
    let r2 = &v; // ✅ 多个只读可以共存
    logln!("两个只读借用：{r1:?} {r2:?}");

    // r1/r2 最后一次使用已过 → 借用结束，下面可以可变借
    let w = &mut v;
    w.push(4);
    logln!("可变借用 push 后：{w:?}");
}

/// NLL：借用活到「最后一次使用」，不是死板到花括号结束。
fn demo_nll() {
    logln!("--- NLL ---");

    let mut s = String::from("hi");
    let r = &s;
    logln!("只读最后一次用：{r}"); // r 的借用到此结束
    let w = &mut s; // ✅ 不再与 r 冲突
    w.push('!');
    logln!("随后可变借用：{w}");
}

/// 用 {{}} 块主动提前结束借用（锁 guard 同款手法）。
fn demo_scope_ends_borrow() {
    logln!("--- 用块提前还借用 ---");

    let mut data = vec![10, 20];
    {
        let view = &data;
        logln!("  块内只读：{view:?}");
    } // view 结束
    data.push(30); // ✅ 块外可以改
    logln!("块外可变：{data:?}");
}

/// 切片：原数据某一段的借用视图（胖指针），不拷贝。细讲见 strings-slices 章。
fn demo_slice_view() {
    logln!("--- 切片是借用视图 ---");

    let v = vec![1, 2, 3, 4, 5];
    let mid: &[i32] = &v[1..4];
    logln!("&v[1..4] = {mid:?}（不拷贝元素）");

    let s = String::from("hello world");
    let word: &str = &s[..5];
    logln!("&s[..5] = {word}");
    logln!("持有切片期间，原数据不能被可变借用或 move 走");
}
