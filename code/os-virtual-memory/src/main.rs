//! 虚拟内存直觉：申请一大块地址空间，但只碰其中一小部分页面
//!
//! 配套文档：docs/concurrency/os-basics.md 「进程隔离且昂贵」
//! 运行：cargo run -p os-virtual-memory（先 cd code）
//!
//! 每个进程有自己的**虚拟地址空间**——它以为自己独占整台机器的内存，OS 在背后做翻译，
//! 真正的物理内存只在你"碰过"（读/写过）的那些页面（通常 4KB 一页）才会被分配。
//! `Vec::with_capacity(n)` 只是告诉操作系统"预留 n 个元素的虚拟地址空间"，
//! 只要不写内容，物理内存基本不涨（很多平台上分配大块内存靠 mmap，
//! 内核会做"按需分页"——真正碰到才建立物理页映射）。
//!
//! 这里故意把容量和"实际碰过的部分"分开打印，量级控制在几十 MB，安全不卡机器。

use labkit::logln;

fn main() {
    logln!("--- 虚拟内存直觉：capacity（预留的虚拟地址空间）vs 真正碰过的字节 ---");

    // 预留 64MB 的容量：这是"告诉 OS 我可能要用这么多虚拟地址空间"，
    // 不代表这 64MB 物理内存立刻就被占用。
    const CAPACITY_MB: usize = 64;
    let capacity_bytes = CAPACITY_MB * 1024 * 1024;

    let mut buf: Vec<u8> = Vec::with_capacity(capacity_bytes);
    logln!(
        "Vec::with_capacity({CAPACITY_MB}MB) 之后：len = {}，capacity = {} 字节（≈{}MB）",
        buf.len(),
        buf.capacity(),
        buf.capacity() / 1024 / 1024
    );
    logln!("★ 此刻 len = 0：一个字节都还没「写」，即使 capacity 已经声明了 {CAPACITY_MB}MB");
    logln!("  操作系统层面，这些虚拟地址大概率还没有映射到真正的物理页——「按需分页」");

    // 只往里写一小部分（1MB），模拟"只碰了一小片"，其余的虚拟地址空间始终没被触碰。
    const TOUCH_MB: usize = 1;
    let touch_bytes = TOUCH_MB * 1024 * 1024;
    buf.resize(touch_bytes, 0xAB);
    // 真正写一遍，确保这些页面被"实打实"地建立了物理映射（不是编译器优化掉的空写）。
    for b in buf.iter_mut() {
        *b = b.wrapping_add(1);
    }
    logln!(
        "resize 并逐字节写入 {TOUCH_MB}MB 之后：len = {} 字节（≈{}MB），capacity 仍是 {} 字节（≈{}MB）",
        buf.len(),
        buf.len() / 1024 / 1024,
        buf.capacity(),
        buf.capacity() / 1024 / 1024
    );

    logln!("★ 直觉小结：");
    logln!("  1) capacity 是「虚拟地址空间的承诺」，len/真正写过的部分才对应「实际用到的物理内存」；");
    logln!("  2) 这也是为什么 Vec::with_capacity 预留一个很大的上限通常很便宜——");
    logln!("     只要不写，代价基本只在虚拟地址空间的登记，不在物理内存；");
    logln!("  3) 每个进程都以为自己独占一整套地址空间——这份「幻觉」正是进程隔离的来源。");

    // 明确 drop，避免编译器认为 buf 没被用到而整个优化掉（教学 demo 的常见坑）。
    drop(buf);
}
