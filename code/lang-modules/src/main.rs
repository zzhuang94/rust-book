//! 语言地基 · 模块、crate 与可见性 —— 可运行示例
//!
//! 配套文档：docs/lang/modules.md
//! 运行：cargo run -p lang-modules（先 cd code）
//!
//! 为了单文件可跑，这里用「内联 mod」演示模块系统；真实项目里
//! 这些 mod 通常各占一个文件（见文档「文件即模块」一节）。

use labkit::logln;

// ---- 一个子模块，演示 pub / 私有 / 嵌套 ----
mod store {
    // pub：对外可见
    pub struct Config {
        pub host: String, // 字段要单独 pub，外面才读得到
        port: u16,        // 私有字段：只有 store 模块内部能访问
    }

    impl Config {
        // pub 关联函数：外面用它来构造（因为 port 私有，外面没法直接写字面量）
        pub fn new(host: &str, port: u16) -> Self {
            Config { host: host.to_string(), port }
        }
        // pub 方法：把私有字段以只读方式暴露
        pub fn port(&self) -> u16 {
            self.port
        }
    }

    pub fn connect(cfg: &Config) -> String {
        // 模块内部可以访问私有的 helper 和私有字段
        format!("{}://{}:{}", scheme(), cfg.host, cfg.port)
    }

    // 私有函数：只有 store 内部能调
    fn scheme() -> &'static str {
        "redis"
    }

    // 再嵌一层子模块，演示 super 回引父模块
    pub mod pool {
        pub fn describe(cfg: &super::Config) -> String {
            // super:: = 上一层父模块（store）
            format!("连接池指向 {}", super::connect(cfg))
        }
    }
}

// ---- 重导出：把深处的类型提到本模块，调用方少写路径 ----
pub use store::Config; // 于是下面可以直接写 Config，而不是 store::Config

fn main() {
    logln!("--- 模块与可见性 ---");

    // 用重导出后的短名字构造（new 里才能访问私有 port）
    let cfg = Config::new("localhost", 6379);
    logln!("  host（pub 字段直接读）= {}", cfg.host);
    logln!("  port（私有字段，靠 pub 方法读）= {}", cfg.port());

    // 调用子模块的 pub 函数：用 crate:: 绝对路径也行，这里用引进来的名字
    logln!("  {}", store::connect(&cfg));

    // 调用嵌套子模块，它内部用 super 回引了父模块
    logln!("  {}", store::pool::describe(&cfg));
}
