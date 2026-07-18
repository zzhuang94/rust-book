//! 文件描述符 / 句柄：进程打开的资源，也是需要"关闭"的东西
//!
//! 配套文档：docs/concurrency/os-basics.md 「进程是运行实例」
//! （进程 = 独立内存空间 + 打开的资源【文件、socket】+ 至少一个线程）
//! 运行：cargo run -p os-file-fd（先 cd code）
//!
//! Linux/macOS 把"打开的文件"叫**文件描述符**（file descriptor，一个小整数，
//! 内核用它在一张表里查到"这个数字对应哪个真正打开的文件"）；
//! Windows 把等价概念叫**句柄**（handle）——概念一致，名字不同、API 不同。
//! Rust 的 `std::fs::File` 把这些平台差异都包起来了：
//! 你拿到一个 `File` 值，它内部持有底层 fd/句柄；`File` 被 drop 时自动关闭。

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

use labkit::logln;

fn main() {
    let path = std::env::temp_dir().join("os_file_fd_demo.txt");
    logln!("--- 文件描述符 / 句柄：用 std::fs::File 打开、读写、关闭 ---");
    logln!("临时文件路径：{}", path.display());

    // 1) 创建并写入：File::create 打开（若不存在则创建，若存在则清空）一个文件，
    //    这一步在内核里登记了一条"打开文件"的记录，Rust 把它包成 File 值返回。
    {
        let mut file = File::create(&path).expect("创建文件失败");
        file.write_all(b"hello from os-file-fd\n").expect("写入失败");
        file.write_all(b"second line\n").expect("写入失败");
        logln!("写入完成，file 值在这个作用域结束时会被 drop —— 底层 fd/句柄随之关闭");
    } // <- file 在这里 drop，对应一次"关闭文件"的系统调用（RAII：不用手写 close）

    // 2) 以读写模式重新打开，演示 seek（在文件内移动读写位置，对应 lseek/SetFilePointer）。
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .expect("重新打开文件失败");

    let mut content = String::new();
    file.read_to_string(&mut content).expect("读取失败");
    logln!("重新打开后读到的内容：\n{content}");

    // seek 回文件开头，追加一段前缀——展示"打开的文件"内部有一个读写位置的概念，
    // 这个位置本身就是内核为这个 fd/句柄维护的状态之一。
    file.seek(SeekFrom::Start(0)).expect("seek 失败");
    file.write_all("[已修改] ".as_bytes()).expect("覆盖写入失败");
    logln!("seek 回开头后覆盖写入了一段前缀");

    drop(file); // 显式 drop，强调"关闭"这一步，虽然作用域结束时也会自动发生

    let final_content = fs::read_to_string(&path).expect("最终读取失败");
    logln!("最终文件内容：\n{final_content}");

    // 清理临时文件。
    fs::remove_file(&path).expect("删除临时文件失败");
    logln!("临时文件已删除");

    logln!("★ 小结：");
    logln!("  - Linux/macOS：这个 File 内部包着一个整数「文件描述符」；");
    logln!("  - Windows：这个 File 内部包着一个「句柄」（HANDLE）；");
    logln!("  - 两者概念等价：都是「内核记录的一份打开资源」，进程持有它就能操作对应文件；");
    logln!("  - Rust 用 RAII 把「关闭」自动化了：File 一旦 drop，底层资源立刻释放，不会泄漏。");
}
