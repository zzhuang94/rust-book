//! 第 00 课 —— 你的第一个 Rust 程序（写给刚装好工具链、只写过 Go 的你）
//!
//! 这个文件不追求"有用"，只追求"把 cargo 跑通、把最小语法摸一遍"。
//! 每一行都有保姆级注释：这行在干嘛、为什么这么写、Go 里对应什么。
//!
//! 运行方式（在 code/ 目录下）：
//!   cargo run -p start-toolchain
//! 其中 `-p hello` 的 hello 就是本 crate 的 package 名（见同目录 Cargo.toml 的 name）。

// `fn main()` 是程序入口，和 Go 的 `func main()` 一模一样：编译成可执行文件后从这里开始跑。
// 区别：Go 里 main 必须在 `package main`；Rust 里「哪个 crate 有 main.rs 里的 fn main，它就是可执行 crate」。
fn main() {
    // ---- 1. 打印一行 ----
    // println! 末尾那个 `!` 说明它是「宏」(macro)，不是普通函数。
    // 为什么是宏？因为它要在编译期检查你的格式串和参数个数是否对得上——这是普通函数做不到的。
    // 对照 Go：fmt.Println("Hello, Rust!")
    println!("Hello, Rust!");

    // ---- 2. 变量：默认不可变 ----
    // let 声明变量。注意：Rust 变量**默认不可变**（immutable），这点和 Go 相反。
    // 下面这行之后，name 就再也不能被改了；想改得写 `let mut name`。
    // 类型没写，编译器会自动推断出 &str（字符串切片）——和 Go 的 := 自动推断类似。
    let name = "Gopher";
    // {name} 是「把同名变量直接插进字符串」的写法（Rust 2021 起支持），
    // 等价于 Go 的 fmt.Printf("你好，%s！\n", name)，但更省事、不用记 %s。
    println!("你好，{name}！欢迎从 Go 转来。");

    // ---- 3. 可变变量：必须显式 mut ----
    // 想让变量可改，必须加 mut。编译器用这个关键字帮你把"打算改"和"不打算改"分得清清楚楚。
    let mut count = 0; // i32（32 位有符号整数），编译器推断得到
    count += 1;
    count += 1;
    // 这里用了另一种占位：{} 是位置占位符，按顺序吃掉后面的参数（对照 Go 的 %v）。
    println!("已经数到 {} 了", count);

    // ---- 4. 函数调用 ----
    // 调用下面自定义的 add 函数。Rust 函数调用和 Go 没差别。
    let sum = add(3, 4);
    println!("3 + 4 = {sum}");

    // ---- 5. 遍历一个集合 ----
    // vec![...] 用宏造一个 Vec<i32>（可增长数组），对照 Go 的 []int{2, 4, 6}。
    let nums = vec![2, 4, 6];
    // for x in &nums：遍历 nums 的「引用」（&），这样循环不会把 nums 的所有权拿走，循环后还能继续用它。
    // 对照 Go：for _, x := range nums { ... }
    for x in &nums {
        println!("  元素：{x}");
    }
    // 因为上面借的是 &nums（只是借看，没拿走），这里 nums 依然可用。
    println!("一共 {} 个元素", nums.len());
}

/// 把两个整数相加并返回。
///
/// 几个和 Go 不一样的点，逐一说清：
/// - 参数类型写在冒号后面：`a: i32`，而不是 Go 的 `a int`（类型在后但无冒号）。
/// - 返回类型用 `->` 标注，这里是 `-> i32`。
/// - 函数体最后一行 `a + b` **没有分号**：在 Rust 里，「不带分号的表达式」就是返回值，
///   等价于 Go 的 `return a + b`。这是 Rust 最容易让 Go 程序员愣住的点之一。
fn add(a: i32, b: i32) -> i32 {
    a + b // 注意：没有分号 = 这就是返回值。写成 `a + b;`（带分号）反而会报错！
}
