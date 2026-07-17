//! 语言地基 · 智能指针全家桶 —— 可运行示例
//!
//! 配套文档：docs/lang/smart-pointers.md
//! 运行：cargo run -p lang-smart-pointers（先 cd code）
//!
//! 故意触发的 RefCell panic / 去 Box 的 E0072 请按文档动手实验自己撞。

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use labkit::logln;

fn main() {
    demo_box_recursive();
    demo_box_trait_object();
    demo_box_deref();
    demo_rc_counts();
    demo_cell();
    demo_refcell();
    demo_rc_refcell();
    demo_cycle_leaks();
    demo_weak_fixes_cycle();
    demo_upgrade_after_drop();
    demo_cow();
    demo_selection_reminder();
}

// ======================== Box ========================

enum Tree {
    Leaf(i64),
    Node(Box<Tree>, Box<Tree>), // 去掉 Box → E0072 infinite size
}

fn sum_tree(t: &Tree) -> i64 {
    match t {
        Tree::Leaf(v) => *v,
        Tree::Node(l, r) => sum_tree(l) + sum_tree(r),
    }
}

fn demo_box_recursive() {
    logln!("--- Box：递归类型 ---");
    let tree = Tree::Node(
        Box::new(Tree::Leaf(3)),
        Box::new(Tree::Node(Box::new(Tree::Leaf(4)), Box::new(Tree::Leaf(5)))),
    );
    logln!("  叶子和 = {}", sum_tree(&tree));
}

trait Greeter {
    fn greet(&self) -> String;
}

struct En;
struct Zh;

impl Greeter for En {
    fn greet(&self) -> String {
        "Hello".into()
    }
}
impl Greeter for Zh {
    fn greet(&self) -> String {
        "你好".into()
    }
}

fn demo_box_trait_object() {
    logln!("--- Box：dyn Trait 异构集合 ---");
    // 不同具体类型，装进同一个 Vec —— 靠 Box<dyn Greeter>
    let crowd: Vec<Box<dyn Greeter>> = vec![Box::new(En), Box::new(Zh)];
    for g in &crowd {
        logln!("  {}", g.greet());
    }
}

fn demo_box_deref() {
    logln!("--- Box：Deref 透明 ---");
    let b = Box::new(String::from("hello"));
    // 不用写 (*b).len()——Deref 让 Box<String> 像 String/&str 用
    logln!("  b.len() = {}，内容 = {b}", b.len());
    takes_str(&b); // deref coercion：&Box<String> → &String → &str
}

fn takes_str(s: &str) {
    logln!("  takes_str 收到：{s}");
}

// ======================== Rc ========================

fn demo_rc_counts() {
    logln!("--- Rc：计数变化 ---");
    let a = Rc::new(String::from("共享数据"));
    logln!("  新建后 strong = {}", Rc::strong_count(&a));

    {
        let b = Rc::clone(&a); // 计数 +1，不复制堆上字符串
        let c = Rc::clone(&a);
        logln!("  clone 两次后 strong = {}（b、c 还在）", Rc::strong_count(&a));
        logln!("  a={a}, b={b}, c={c}");
    } // b、c drop → 计数回到 1
    logln!("  内层结束后 strong = {}", Rc::strong_count(&a));
}

// ======================== Cell / RefCell ========================

fn demo_cell() {
    logln!("--- Cell：Copy 小值内部可变 ---");
    let counter = Cell::new(0);
    bump(&counter);
    bump(&counter);
    logln!("  通过 &Cell 改了两次，现在 = {}", counter.get());
}

fn bump(c: &Cell<i32>) {
    c.set(c.get() + 1); // 整存整取，无借用、永不因借用 panic
}

fn demo_refcell() {
    logln!("--- RefCell：运行期借用检查 ---");
    let cell = RefCell::new(vec![1, 2, 3]);
    {
        let mut w = cell.borrow_mut();
        w.push(4);
        logln!("  borrow_mut 期间：{w:?}");
    } // w drop → 写借用归还
    logln!("  归还后 borrow：{:?}", cell.borrow());

    // 错误示范（请自己取消注释体验 panic）：
    // let _r = cell.borrow();
    // let _w = cell.borrow_mut(); // BorrowMutError
}

// ======================== Rc<RefCell> ========================

fn demo_rc_refcell() {
    logln!("--- Rc<RefCell>：共享且可改 ---");
    let shared = Rc::new(RefCell::new(vec![1, 2, 3]));
    let also = Rc::clone(&shared);
    also.borrow_mut().push(4);
    logln!("  另一句柄改完，这边看到：{:?}", shared.borrow());
    logln!("  strong_count = {}", Rc::strong_count(&shared));
}

// ======================== 环与 Weak ========================

/// 错误示范：父子互指都用 Rc → 外界放手后仍泄漏（Drop 不会跑）。
struct LeakyNode {
    value: i64,
    parent: RefCell<Option<Rc<LeakyNode>>>,
    children: RefCell<Vec<Rc<LeakyNode>>>,
}

impl Drop for LeakyNode {
    fn drop(&mut self) {
        logln!("  [LeakyNode] {} 被释放（若你看到这行，说明没泄漏）", self.value);
    }
}

fn demo_cycle_leaks() {
    logln!("--- 反例：Rc 互指会泄漏 ---");
    {
        let parent = Rc::new(LeakyNode {
            value: 1,
            parent: RefCell::new(None),
            children: RefCell::new(vec![]),
        });
        let child = Rc::new(LeakyNode {
            value: 2,
            parent: RefCell::new(Some(Rc::clone(&parent))), // ❌ 强回指
            children: RefCell::new(vec![]),
        });
        parent.children.borrow_mut().push(Rc::clone(&child));

        logln!(
            "  环建成后：parent.strong={} child.strong={}（child 强回指父）",
            Rc::strong_count(&parent),
            Rc::strong_count(&child)
        );
        logln!(
            "  child.parent 是否 Some？ {}",
            child.parent.borrow().is_some()
        );
        logln!("  即将放下 parent/child 变量……（下面不应出现 Drop 打印）");
    }
    logln!("  块已结束：若上面没有「被释放」，就是泄漏了（计数卡在环里）");
}

/// 正确：回指用 Weak。
struct Node {
    value: i64,
    parent: RefCell<Weak<Node>>,
    children: RefCell<Vec<Rc<Node>>>,
}

impl Drop for Node {
    fn drop(&mut self) {
        logln!("  [Node] {} 被释放", self.value);
    }
}

fn demo_weak_fixes_cycle() {
    logln!("--- Weak：破环，正常释放 ---");
    {
        let parent = Rc::new(Node {
            value: 10,
            parent: RefCell::new(Weak::new()),
            children: RefCell::new(vec![]),
        });
        let child = Rc::new(Node {
            value: 20,
            parent: RefCell::new(Rc::downgrade(&parent)), // ✅ 弱回指
            children: RefCell::new(vec![]),
        });
        parent.children.borrow_mut().push(Rc::clone(&child));

        logln!(
            "  strong(parent)={} weak(parent)={} strong(child)={}",
            Rc::strong_count(&parent),
            Rc::weak_count(&parent),
            Rc::strong_count(&child)
        );

        match child.parent.borrow().upgrade() {
            Some(p) => logln!("  child 的父 = {}", p.value),
            None => logln!("  父已不在"),
        }
        logln!("  块结束，应看到两个 Node 被释放：");
    }
}

fn demo_upgrade_after_drop() {
    logln!("--- upgrade：父已死后得 None ---");
    let weak: Weak<Node>;
    {
        let parent = Rc::new(Node {
            value: 100,
            parent: RefCell::new(Weak::new()),
            children: RefCell::new(vec![]),
        });
        weak = Rc::downgrade(&parent);
        logln!("  父还在，upgrade = {:?}", weak.upgrade().map(|p| p.value));
    } // parent drop
    logln!(
        "  父已死，upgrade = {:?}（Weak 不阻止释放）",
        weak.upgrade().map(|p| p.value)
    );
}

// ======================== Cow ========================

fn normalize(input: &str) -> Cow<'_, str> {
    if input.contains(' ') {
        Cow::Owned(input.replace(' ', "_"))
    } else {
        Cow::Borrowed(input)
    }
}

fn demo_cow() {
    logln!("--- Cow：多半借用、偶尔拥有 ---");
    let a = normalize("no_space");
    let b = normalize("有 空 格");
    logln!(
        "  no_space → {a}，是借用? {}",
        matches!(a, Cow::Borrowed(_))
    );
    logln!(
        "  有空格 → {b}，是拥有? {}",
        matches!(b, Cow::Owned(_))
    );
    // Cow 自动 Deref 成 &str
    logln!("  deref 后长度：a={} b={}", a.len(), b.len());
}

fn demo_selection_reminder() {
    logln!("--- 选型速记 ---");
    logln!("  上堆/递归/dyn → Box");
    logln!("  单线程共享 → Rc；跨线程 → Arc（见共享状态章）");
    logln!("  共享还要改 → Rc<RefCell> 或 Arc<Mutex>");
    logln!("  回指/观察者 → Weak，防泄漏");
    logln!("  多半不改偶要改 → Cow");
}
