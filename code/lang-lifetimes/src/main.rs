//! 语言地基 · 生命周期 —— 可运行示例
//!
//! 配套文档：docs/lang/lifetimes.md
//! 运行：cargo run -p lang-lifetimes（先 cd code）
//!
//! 只含能编过的正面示例；悬垂引用等反例见文档，请亲手撞 E0597。

use labkit::logln;

fn main() {
    demo_longest_ok();
    demo_elision_equivalent();
    demo_struct_holds_ref();
    demo_static_str();
    demo_two_lifetimes();
    demo_method_elision();
    demo_own_instead();
}

/// longest：返回的引用有效期不超过两个参数里较短的那个。
fn demo_longest_ok() {
    logln!("--- longest 合法调用 ---");

    let s1 = String::from("长长的字符串");
    let s2 = String::from("短");
    // s1、s2 都活过 result 的使用点 → 说明书满意
    let result = longest(s1.as_str(), s2.as_str());
    logln!("较长的是：{result}");
}

/// 说明书：返回值借自 x 或 y，所以有效期 ≤ 二者较短者。
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() >= y.len() {
        x
    } else {
        y
    }
}

/// 省略规则：只有一个引用参数时，返回引用自动跟它走——手写 'a 完全等价。
fn demo_elision_equivalent() {
    logln!("--- 省略规则等价 ---");

    let s = String::from("hello world");
    let w1 = first_word_elided(&s);
    let w2 = first_word_explicit(&s);
    logln!("省略版={w1}，显式版={w2}（应相同）");
}

fn first_word_elided(s: &str) -> &str {
    match s.find(' ') {
        Some(i) => &s[..i],
        None => s,
    }
}

fn first_word_explicit<'a>(s: &'a str) -> &'a str {
    match s.find(' ') {
        Some(i) => &s[..i],
        None => s,
    }
}

/// 结构体存引用：必须声明生命周期 = 承诺「我活不过我借的数据」。
fn demo_struct_holds_ref() {
    logln!("--- 结构体持有引用 ---");

    let novel = String::from("Call me Ishmael. Some years ago...");
    let first_sentence = novel.split('.').next().unwrap();
    let excerpt = Excerpt { part: first_sentence };
    logln!("摘录：{}", excerpt.part);
    // excerpt 不能活过 novel（编译器按 'a 查账）
}

struct Excerpt<'a> {
    part: &'a str,
}

/// 'static：活到程序结束。字符串字面量直接编进二进制。
fn demo_static_str() {
    logln!("--- 'static ---");

    let s: &'static str = "编进二进制的字面量";
    logln!("'static 字符串：{s}");

    // 需要把「拥有的 String」变成 'static 引用？做不到（除非泄漏或改设计）。
    // spawn 要 'static 时：move 所有权进去，或用 Arc 共享——见异步章节。
}

/// 两个生命周期：返回值只跟其中一个参数走时，别强行写成同一个 'a。
fn demo_two_lifetimes() {
    logln!("--- 两个生命周期参数 ---");

    let left = String::from("左边很长很长");
    {
        let tip = "提示"; // 字面量 'static，很耐活
        // 返回借自 first 的那份；second 只是陪跑，不必同寿
        let chosen = first_or_hint(left.as_str(), tip);
        logln!("选中：{chosen}");
    }
}

fn first_or_hint<'a, 'b>(first: &'a str, _hint: &'b str) -> &'a str {
    // 返回值只标注 'a：明确说「我只可能借自 first」
    first
}

/// 方法里有 &self 时，返回的引用默认借自 self（省略规则 3）。
fn demo_method_elision() {
    logln!("--- 方法省略 ---");

    let book = Book {
        title: String::from("Rust 手册"),
        body: String::from("Chapter 1: ownership..."), // 用 ASCII，避免按字节切片切到 UTF-8 中间
    };
    let t = book.title_ref();
    logln!("书名借用：{t}");
    logln!("正文前缀：{}", book.preview(10));
}

struct Book {
    title: String,
    body: String,
}

impl Book {
    // 等价于 fn title_ref<'a>(&'a self) -> &'a str
    fn title_ref(&self) -> &str {
        &self.title
    }

    fn preview(&self, n: usize) -> &str {
        let end = n.min(self.body.len());
        &self.body[..end]
    }
}

/// 初学解药：让数据拥有所有权，少在结构体/返回值里玩引用。
fn demo_own_instead() {
    logln!("--- 拥有比借用更省心 ---");

    let owned = ExcerptOwned {
        part: String::from("我自己持有这段文字"),
    };
    logln!("无生命周期标注的结构体：{}", owned.part);
}

struct ExcerptOwned {
    part: String, // 拥有 → 结构体不用写 <'a>
}
