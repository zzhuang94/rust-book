//! 语言地基 · 类型系统与 trait —— 可运行示例
//!
//! 配套文档：docs/lang/types-traits.md
//! 运行：cargo run -p lang-types-traits（先 cd code）

use labkit::logln;

fn main() {
    demo_struct();
    demo_enum_match();
    demo_generics();
    demo_trait_default();
    demo_dispatch();
    demo_from();
}

// ============ struct 与三种 self ============

struct User {
    name: String,
    age: u8,
}

impl User {
    fn new(name: impl Into<String>) -> Self {
        // 关联函数（无 self）≈ 构造函数；impl Into<String> 让调用方传 &str 或 String 都行
        User { name: name.into(), age: 0 }
    }
    fn greet(&self) -> String {
        // &self：只读借用自己
        format!("我是 {}", self.name)
    }
    fn birthday(&mut self) {
        // &mut self：可变借用自己，能改字段
        self.age += 1;
    }
    fn into_name(self) -> String {
        // self：拿走所有权，调用后这个 User 就没了
        self.name
    }
}

fn demo_struct() {
    logln!("--- struct 与三种 self ---");
    let mut u = User::new("张三"); // 传 &str，内部 .into() 成 String
    logln!("{}", u.greet()); // &self
    u.birthday(); // &mut self
    logln!("过完生日 age = {}", u.age);
    let name = u.into_name(); // self：消费掉 u
    logln!("拆出 name = {name}（u 已被消费，不能再用）");
}

// ============ enum + match ============

/// 一条命令：每个变体带的数据形状都不一样。
enum Command {
    Get(String),
    Set { key: String, ttl: Option<u64> },
    Quit,
}

fn run(cmd: Command) {
    // match 强制覆盖所有变体；漏一个就编译不过
    match cmd {
        Command::Quit => logln!("  再见"),
        Command::Get(key) => logln!("  读 {key}"), // 解构出 String
        Command::Set { key, ttl: Some(t), .. } => logln!("  写 {key}，ttl={t}"), // 嵌套匹配 + 守卫式解构
        Command::Set { key, ttl: None, .. } => logln!("  永久写 {key}"),
    }
}

fn demo_enum_match() {
    logln!("--- enum + match ---");
    run(Command::Get("user:1".into()));
    run(Command::Set { key: "token".into(), ttl: Some(60) });
    run(Command::Set { key: "cfg".into(), ttl: None });
    run(Command::Quit);
}

// ============ 泛型 ============

/// 泛型函数：对任何"能比大小"的类型 T 都适用。约束 T: PartialOrd。
fn largest<T: PartialOrd>(list: &[T]) -> &T {
    let mut max = &list[0];
    for x in list {
        if x > max {
            max = x;
        }
    }
    max
}

fn demo_generics() {
    logln!("--- 泛型（单态化，零成本）---");
    // 编译器会为 i32 和 &str 各生成一份专属代码
    logln!("最大的数 = {}", largest(&[3, 7, 2, 9, 4]));
    logln!("最大的词 = {}", largest(&["pear", "apple", "kiwi"]));
}

// ============ trait 默认方法 + 分发 ============

/// 一个招呼 trait：name 必须实现，greet 有默认实现。
trait Greeter {
    fn name(&self) -> String;
    fn greet(&self) -> String {
        // 默认方法：实现者不写也白得
        format!("你好，我是 {}", self.name())
    }
}

struct Cat;
struct Dog;

impl Greeter for Cat {
    fn name(&self) -> String {
        "猫".into()
    } // 只写 name，白用默认 greet
}
impl Greeter for Dog {
    fn name(&self) -> String {
        "狗".into()
    }
    fn greet(&self) -> String {
        "汪！".into()
    } // 覆盖默认实现
}

fn demo_trait_default() {
    logln!("--- trait 默认方法 ---");
    logln!("{}", Cat.greet()); // 用默认实现
    logln!("{}", Dog.greet()); // 用覆盖后的实现
}

/// 静态分发：impl Trait，编译期定死类型，零开销。
fn print_static(g: impl Greeter) {
    logln!("  [静态] {}", g.greet());
}

fn demo_dispatch() {
    logln!("--- 静态分发 vs 动态分发 ---");

    // 静态：各调各的专属版本
    print_static(Cat);
    print_static(Dog);

    // 动态：一个 Vec 装下不同类型（Go interface 那种），运行期查 vtable
    let zoo: Vec<Box<dyn Greeter>> = vec![Box::new(Cat), Box::new(Dog)];
    for g in &zoo {
        logln!("  [动态] {}", g.greet());
    }
}

// ============ From / Into ============

/// 一个自定义错误，能从 io 错误自动转换而来。
#[derive(Debug)]
struct AppError(String);

impl From<std::io::Error> for AppError {
    // 实现了 From，? 运算符就能自动把 io::Error 转成 AppError
    fn from(e: std::io::Error) -> Self {
        AppError(format!("io 出错: {e}"))
    }
}

/// 打开一个不存在的文件，让 ? 把 io::Error 自动转成 AppError。
fn open_missing() -> Result<String, AppError> {
    let _f = std::fs::File::open("绝对不存在的文件.txt")?; // ? 处失败 → From 自动转 → return Err(AppError)
    Ok("不会走到这".into())
}

fn demo_from() {
    logln!("--- From/Into：? 自动转错误 ---");
    match open_missing() {
        Ok(_) => logln!("居然成功了？"),
        Err(e) => logln!("如期失败，错误被自动转换为 AppError: {}", e.0),
    }
}
