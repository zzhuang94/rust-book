//! 语言地基 · 函数与闭包 —— 可运行示例
//!
//! 配套文档：docs/lang/functions-closures.md
//! 运行：cargo run -p lang-functions-closures（先 cd code）
//!
//! 前置心智：docs/lang/ownership.md（move / 借用）。闭包捕获就是所有权规则的投影。

use labkit::logln;

fn main() {
    demo_fn_basics();
    demo_expression_return();
    demo_pass_by_value_and_ref();
    demo_closure_syntax();
    demo_capture_immut();
    demo_capture_mut();
    demo_capture_move();
    demo_fn_traits();
    demo_closure_as_callback();
    demo_async_move_intuition();
}

/// 函数：参数类型在冒号后，返回类型用 ->；和 Go 很像，标点不同。
fn demo_fn_basics() {
    logln!("--- 函数基础 ---");

    let s = add(3, 4);
    logln!("add(3, 4) = {s}");

    greet("Gopher"); // 无返回值 ≈ Go 的无返回 func
}

fn add(a: i32, b: i32) -> i32 {
    a + b // 见下一节：无分号 = 返回值
}

fn greet(name: &str) {
    logln!("你好，{name}！");
}

/// 最后一行不带分号的表达式就是返回值；带了分号反而变成返回 ()。
fn demo_expression_return() {
    logln!("--- 表达式返回 ---");

    let a = answer_ok();
    logln!("answer_ok() = {a}");

    // 早期 return 仍然用 return 关键字（尤其在 ? 或卫语句场景）
    logln!("early_return(true) = {}", early_return(true));
    logln!("early_return(false) = {}", early_return(false));
}

fn answer_ok() -> i32 {
    42 // ✅ 无分号
    // 若写成 42;  → 函数返回 ()，与 -> i32 冲突，编译错误
}

fn early_return(flag: bool) -> i32 {
    if flag {
        return 1; // 显式提前返回，这里分号是语句的一部分
    }
    0 // 正常路径仍可用表达式返回
}

/// 传参：默认按 move（或 Copy）；想借用就写 & / &mut——签名即契约。
fn demo_pass_by_value_and_ref() {
    logln!("--- 传参：值 / 借用 ---");

    let name = String::from("Rust");
    // take_owner(name); 之后就不能再用 name —— 所有权交进去了
    // 这里演示借用：只读看一眼，所有权留在本函数
    print_len(&name);
    logln!("借用后 name 仍在：{name}");

    let mut score = 10;
    bump(&mut score);
    logln!("可变借用改完 score = {score}");
}

fn print_len(s: &String) {
    logln!("  长度（字节）= {}", s.len());
}

fn bump(n: &mut i32) {
    *n += 1; // * 解引用后修改指向的值
}

/// 闭包：|参数| 表达式；类型常可推断；可写成块。
fn demo_closure_syntax() {
    logln!("--- 闭包语法 ---");

    let add = |a: i32, b: i32| a + b; // 和 fn 很像，可省返回类型
    logln!("闭包 add(2, 3) = {}", add(2, 3));

    let add_infer = |a, b| a + b; // 参数类型由第一次调用推断
    logln!("推断版 add_infer(2, 3) = {}", add_infer(2, 3));
    // 注意：推断后类型固定，不能再拿去加字符串

    let describe = |n: i32| {
        // 多行：块的最后一个表达式是返回值
        let kind = if n % 2 == 0 { "偶" } else { "奇" };
        format!("{n} 是{kind}数")
    };
    logln!("{}", describe(4));
}

/// 捕获不可变借用：闭包默认「能借就借」，外面的变量循环后还能用。
fn demo_capture_immut() {
    logln!("--- 捕获：不可变借用 ---");

    let prefix = String::from("日志");
    let log = |msg: &str| {
        // 这里捕获的是 &prefix（只读借），没有抢走 prefix
        logln!("  [{prefix}] {msg}");
    };
    log("启动");
    log("就绪");
    logln!("闭包用完，prefix 仍在：{prefix}");
}

/// 捕获可变借用：闭包里要改外部环境，环境变量得是 mut，且同一时间只能有一个这种闭包在用。
fn demo_capture_mut() {
    logln!("--- 捕获：可变借用 ---");

    let mut count = 0;
    let mut tick = || {
        count += 1; // 可变借用 count
        logln!("  count = {count}");
    };
    tick();
    tick();
    // 这里 tick 还活着时，不能同时再借 count（借用规则）
    drop(tick); // 显式结束闭包，释放对 count 的借用
    logln!("结束后 count = {count}");
}

/// move 闭包：把捕获变量的所有权搬进闭包——外面不能再用。
fn demo_capture_move() {
    logln!("--- 捕获：move ---");

    let name = String::from("异步任务");
    let task = move || {
        // name 的所有权进了闭包；闭包可以活得比当前作用域更久
        logln!("  任务持有：{name}");
    };
    task();
    // logln!("{name}"); // ❌ value borrowed here after move
    logln!("name 已搬进闭包，外面不能再用（这正是 spawn 要的）");
}

/// Fn / FnMut / FnOnce：闭包能调用几次、能不能改捕获，由这三个 trait 描述。
fn demo_fn_traits() {
    logln!("--- Fn / FnMut / FnOnce ---");

    // Fn：可反复调用，只不可变借用捕获（或没捕获）
    let hello = || logln!("  Fn：hello");
    call_twice(&hello);

    // FnMut：可反复调用，但可能可变借用捕获
    let mut n = 0;
    let mut inc = || {
        n += 1;
        logln!("  FnMut：n={n}");
    };
    call_twice_mut(&mut inc);

    // FnOnce：只能调用一次——典型是消费了捕获的所有权
    let token = String::from("一次性令牌");
    let consume = move || {
        logln!("  FnOnce：吃掉 {token}");
        // token 被 move 进闭包并在此结束生命周期 → 只能 call 一次
    };
    consume();
    // consume(); // ❌ 用了 move 进来的 String，闭包已变成 FnOnce，第二次编译不过
}

fn call_twice<F>(f: &F)
where
    F: Fn(),
{
    f();
    f();
}

fn call_twice_mut<F>(f: &mut F)
where
    F: FnMut(),
{
    f();
    f();
}

/// 闭包当回调：map / filter 的日常；也常作为函数参数（泛型 + Fn trait）。
fn demo_closure_as_callback() {
    logln!("--- 闭包当回调 ---");

    let nums = vec![1, 2, 3, 4, 5];
    let doubled: Vec<i32> = nums.iter().map(|x| x * 2).collect();
    logln!("map 加倍：{doubled:?}");

    let sum = apply_twice(5, |x| x + 3); // 传一个闭包进去
    logln!("apply_twice(5, +3) = {sum}");
}

fn apply_twice<F>(x: i32, f: F) -> i32
where
    F: Fn(i32) -> i32,
{
    f(f(x))
}

/// 直觉预告：tokio::spawn(async move { ... }) 里的 move 和闭包 move 是同一类问题。
fn demo_async_move_intuition() {
    logln!("--- async move 直觉（同步模拟）---");

    // 真实代码是：tokio::spawn(async move { use name; });
    // 这里用「把闭包存起来稍后再调用」模拟「任务可能活得更久」
    let name = String::from("worker");
    let job = move || {
        logln!("  延迟执行的任务：我是 {name}");
    };

    // 假装「把任务交出去」——交出去之后，当前栈帧随时可能结束
    run_later(job);
}

fn run_later<F: FnOnce()>(job: F) {
    logln!("  （调度器）稍后运行任务…");
    job();
}
