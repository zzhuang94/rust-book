//! 磁盘 IO：每次 write 都落地 vs 攒一批再 flush
//!
//! 配套文档：docs/concurrency/os-basics.md
//! 运行：cargo run -p os-disk-io（先 cd code）
//!
//! 每一次 `write` + `flush` 都是一次系统调用（往下还可能牵扯真正的磁盘/SSD IO），
//! 系统调用要在"用户态"和"内核态"之间切换一次，本身就有固定开销（跟"用户态/内核态"
//! 那一节呼应）。这里用最朴素的方式对比两种写法：
//!
//!   A) 逐次写入：每写一行就 flush 一次 —— N 行 = N 次"落地"系统调用；
//!   B) 批量写入：先在内存里拼好整块数据，一次性写完再 flush 一次 —— 只有 1 次落地。
//!
//! 临时文件放在 std::env::temp_dir()，程序结束前手动删除，不留垃圾。

use std::fs::{self, File};
use std::io::Write;
use std::time::Instant;

use labkit::logln;

const LINES: usize = 20_000;

fn line_content(i: usize) -> String {
    format!("这是第 {i} 行数据，用来把文件撑出一定大小\n")
}

/// 方式 A：逐次写入 —— 每写一行就 flush，模拟"每次 write 都要落地"的密集调用场景。
fn write_per_line(path: &std::path::Path) -> std::time::Duration {
    let mut file = File::create(path).expect("创建临时文件失败");
    let t = Instant::now();
    for i in 0..LINES {
        file.write_all(line_content(i).as_bytes()).expect("write 失败");
        file.flush().expect("flush 失败"); // 强制这一行立刻"落地"，而不是等系统缓冲区攒够
    }
    t.elapsed()
}

/// 方式 B：先在内存里拼好整块数据，一次 write + 一次 flush。
fn write_batched(path: &std::path::Path) -> std::time::Duration {
    let mut buf = String::with_capacity(LINES * 32);
    for i in 0..LINES {
        buf.push_str(&line_content(i));
    }
    let mut file = File::create(path).expect("创建临时文件失败");
    let t = Instant::now();
    file.write_all(buf.as_bytes()).expect("write 失败");
    file.flush().expect("flush 失败");
    t.elapsed()
}

fn main() {
    let dir = std::env::temp_dir();
    let path_a = dir.join("os_disk_io_demo_per_line.txt");
    let path_b = dir.join("os_disk_io_demo_batched.txt");

    logln!("--- 磁盘 IO：逐次 flush vs 批量一次 flush（各写 {LINES} 行） ---");
    logln!("临时文件目录：{}", dir.display());

    let dur_a = write_per_line(&path_a);
    logln!("方式 A（每行一次 write+flush）耗时: {dur_a:?}");

    let dur_b = write_batched(&path_b);
    logln!("方式 B（拼好整块，一次 write+flush）耗时: {dur_b:?}");

    if dur_a > dur_b {
        let ratio = dur_a.as_secs_f64() / dur_b.as_secs_f64().max(1e-9);
        logln!("★ 逐次 flush 慢了约 {ratio:.1}x —— 差的是 {LINES} 次系统调用的固定开销，不是磁盘带宽");
    } else {
        logln!("★ 本次没测出明显差异（可能命中了系统页缓存/SSD 很快），但结论依然成立：");
        logln!("  减少系统调用次数几乎总是划算的，这也是 BufWriter 存在的理由");
    }

    // 清理临时文件，别留垃圾。
    let _ = fs::remove_file(&path_a);
    let _ = fs::remove_file(&path_b);
    logln!("临时文件已清理");
}
