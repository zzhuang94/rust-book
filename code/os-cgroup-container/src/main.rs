//! 是不是在容器里跑：读几个"约定路径"，读不到就说明是普通进程
//!
//! 配套文档：docs/concurrency/os-basics.md 「进程是运行实例」「进程隔离且昂贵」
//! 运行：cargo run -p os-cgroup-container（先 cd code）
//!
//! 重要前提：**容器不是一种新的"运行单元"**，容器里跑的仍然是普通的 OS 进程；
//! Docker/Kubernetes 之类的"容器"，本质是给这个普通进程叠了几层内核机制：
//!   - **namespace**：让进程看到一份"裁剪过"的视图（自己的进程列表、网络接口、文件系统根……）；
//!   - **cgroup**（control group）：给进程（或进程组）设置资源限额（CPU、内存上限……）。
//! 这份 demo 只做只读检测，完全没有特权操作、不修改任何系统状态：
//!   1. Linux 上有个几乎所有容器运行时都会创建的"标志文件" `/.dockerenv`；
//!   2. Linux 上读 `/proc/1/cgroup`，容器里这个文件的内容通常带 docker/kubepods 之类字样；
//!   3. 两者都读不到（比如这就是 Windows，或是一台裸机 Linux）——说明是"普通进程"。
//! 最后再打印一次 available_parallelism，呼应 os-computer-basics 那一课，
//! 提醒一句：容器里看到的"核数"，很可能是 cgroup 限额后的数字，不一定等于物理机核数。

use std::path::Path;

use labkit::logln;

fn detect_dockerenv() -> bool {
    Path::new("/.dockerenv").exists()
}

/// 读 /proc/1/cgroup（1 号进程的 cgroup 信息），看看里面有没有容器运行时的痕迹。
/// 这个文件只存在于 Linux；Windows 上直接读不到，属于正常情况，不是错误。
fn detect_cgroup_hint() -> Option<String> {
    let content = std::fs::read_to_string("/proc/1/cgroup").ok()?;
    let hit = content
        .lines()
        .find(|line| line.contains("docker") || line.contains("kubepods") || line.contains("containerd"));
    hit.map(|line| line.to_string())
}

fn main() {
    logln!("--- 检测「是不是在容器里跑」（只读检测，无特权，无破坏） ---");

    let mut looks_like_container = false;

    if detect_dockerenv() {
        logln!("发现 /.dockerenv 存在 —— 这是容器运行时常见的标志文件");
        looks_like_container = true;
    } else {
        logln!("/.dockerenv 不存在（Windows 上必然如此；裸机 Linux 上也应该没有）");
    }

    match detect_cgroup_hint() {
        Some(line) => {
            logln!("读到 /proc/1/cgroup 里的可疑行：{line}");
            looks_like_container = true;
        }
        None => {
            logln!("没能从 /proc/1/cgroup 读到容器痕迹（这台机器不是 Linux，或者本来就不在容器里）");
        }
    }

    if looks_like_container {
        logln!("★ 综合判断：这个进程很可能跑在容器里");
    } else {
        logln!("★ 综合判断：没有发现容器痕迹 —— 这就是一个「普通进程」，直接跑在宿主 OS 上");
    }

    // 呼应 os-computer-basics：容器里的这个数字可能是 cgroup CPU 限额换算出来的"核数"，
    // 不一定等于物理机真实核数——这也是"容器只是叠了限额层的普通进程"的一个具体体现。
    match std::thread::available_parallelism() {
        Ok(n) => logln!(
            "available_parallelism() = {n} —— 如果这是容器，这个数字可能受 CPU 限额（cgroup）影响，不等于宿主机物理核数"
        ),
        Err(e) => logln!("available_parallelism() 查询失败：{e}"),
    }

    logln!("★ 小结：容器不是一种新的调度单元，进程、线程、调度那一套完全没变——");
    logln!("  它只是给一个普通进程加了 namespace（视图隔离）+ cgroup（资源限额）两层内核机制。");
}
