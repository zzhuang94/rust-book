# 一切皆文件描述符

> 代码：`code/os-file-fd/`　运行：`cargo run -p os-file-fd`

前置：[《进程与线程》](process-thread.md)（进程、线程、阻塞）。  
这一课把「文件」这个词从「磁盘上的 `.txt`」扩展成操作系统眼里的通用概念：  
**任何能读写的东西，内核都发给你一个小整数当门票**——这个整数就是  
**文件描述符（file descriptor，简称 fd）**。磁盘文件、终端、管道，  
后面 [《socket 就是文件描述符》](../network/socket.md) 要讲的网络连接，  
在 Unix 世界里全部走这一套。

Windows 上对应的东西叫 **句柄（handle）**，思路完全一样、名字不同，  
本课统一用 fd 讲，Windows 细节随手带一句。

----

# fd 只是一个小整数

> 第一个要打破的直觉：fd **不是**指针、不是文件名，只是一个从 0 开始  
> 数上去的普通整数。

打开一个文件，操作系统不会把整个文件内容或者一大堆元信息直接甩给你，  
而是返回一个整数，比如 `3`。这个整数本身什么都不是——它只是一个  
**索引**，用来去查内核里一张真正保存信息的表。

```rust
use std::fs::File;
use std::os::fd::AsRawFd; // Unix 平台才有；Windows 用 AsRawHandle

let f = File::open("Cargo.toml")?;
println!("这个文件的 fd 是: {}", f.as_raw_fd()); // 很可能打印 3
```

Go 里同样能看到这个整数（虽然日常代码几乎不需要）：

```go
f, _ := os.Open("go.mod")
fmt.Println("这个文件的 fd 是:", f.Fd())
```

**为什么“只是个整数”这件事重要**：因为它解释了 fd 为什么能被  
`dup`（复制）、能被子进程继承、能在 `select`/`epoll` 的位图里当索引用——  
一个整数比一个真正的文件对象轻得多，这是整套设计的起点。

----

# 每个进程有自己的表

> fd 这个整数不是全局唯一的，它只在**当前进程**里有意义——  
> 这是新手最容易搞混的一点。

内核给每个进程维护一张私有的 **打开文件表（open file descriptor table）**，  
大概长这样：

```
进程 A 的 fd 表          进程 B 的 fd 表
┌────┬──────────┐        ┌────┬──────────┐
│ 0  │ → 终端输入 │        │ 0  │ → 终端输入 │
│ 1  │ → 终端输出 │        │ 1  │ → 日志文件 │
│ 2  │ → 终端输出 │        │ 2  │ → 终端输出 │
│ 3  │ → config.txt│       │ 3  │ → data.db  │
└────┴──────────┘        └────┴──────────┘
```

- 进程 A 的 `fd = 3` 和进程 B 的 `fd = 3` **完全无关**，可能指向  
  两个毫不相干的文件——这就是为什么“fd 3 打开失败”这种报错，  
  一定要连着“哪个进程”一起看才有意义；
- 表里每一项不是文件内容本身，而是一个指针，指向内核里**真正的**  
  文件状态结构（当前读写位置、打开模式等）——这一层indirection  
  是 `dup` 能实现“两个 fd 共享同一个读写位置”的关键，后面详讲。

🔬 底层视角：Linux 下可以直接“看见”这张表——  
`ls -l /proc/<pid>/fd/`，会列出该进程当前打开的每一个 fd 及其指向的  
文件/管道/socket。下次遇到“文件描述符泄漏”的排障场景，这条命令  
比猜代码快得多。

----

# 三个天生打开的流

> 每个进程一出生，fd 表里就已经有三项了，不用你手动打开。

- **fd 0 = 标准输入（stdin）**：默认接终端键盘输入，也可以被  
  重定向成文件或管道的内容；
- **fd 1 = 标准输出（stdout）**：`println!`/`fmt.Println` 最终都写向  
  这里，默认显示在终端；
- **fd 2 = 标准错误（stderr）**：日志、错误信息的传统去处，  
  和 stdout **是两条独立的流**，即便默认都显示在同一个终端上。

```rust
use std::io::Write;

println!("这行走 stdout（fd 1）");
eprintln!("这行走 stderr（fd 2）");
// 手动写 fd 2 的等价方式：
std::io::stderr().write_all(b"也是 fd 2\n")?;
```

```go
fmt.Println("这行走 stdout（fd 1）")           // os.Stdout
fmt.Fprintln(os.Stderr, "这行走 stderr（fd 2）") // 显式指定 fd 2
```

**为什么要把 stdout/stderr 分开**：命令行里可以分别重定向——

```bash
./myapp > out.log 2> err.log     # 正常输出和错误分别存文件
./myapp > all.log 2>&1           # 2>&1：让 fd 2 也指向 fd 1 当前指向的地方
```

`2>&1` 这行命令直接对应上一节的“fd 表”模型：它不是复制文件内容，  
而是让 fd 2 这一项，和 fd 1 这一项，指向内核里**同一份**文件状态——  
这正是下一节 `dup` 要讲的机制。

----

# open 到 close 的四步

> 忘掉高级封装，看一眼这几个系统调用本来的样子——  
> Rust/Go 的标准库最终都落到它们上面。

Unix 系统调用视角的“操作一个文件”，永远是这四步：

```
open(路径, 模式)  → 拿到一个新 fd（内核在打开文件表里新开一项）
read(fd, buf)     → 从当前读写位置读一段，位置自动往前挪
write(fd, buf)    → 从当前读写位置写一段，位置自动往前挪
close(fd)         → 归还这个 fd，内核回收对应的表项
```

Rust 标准库把这四步包进了 `File` 类型，`Drop` 时自动帮你 `close`：

```rust
use std::fs::File;
use std::io::{Read, Write};

let mut f = File::create("demo.txt")?;   // open（创建/覆盖模式）
f.write_all(b"hello fd\n")?;             // write
drop(f);                                  // close（其实作用域结束会自动做）

let mut f = File::open("demo.txt")?;     // open（只读模式）
let mut buf = String::new();
f.read_to_string(&mut buf)?;             // read（读到 EOF 为止）
println!("读到: {buf}");
```

Go 的 `os.File` 是同一套心智，`defer f.Close()` 是雷打不动的习惯：

```go
f, _ := os.Create("demo.txt")
defer f.Close()
f.WriteString("hello fd\n")
```

**忘记 close 会怎样**：fd 不会立刻被回收，会一直占着这张表里的一项。  
进程能打开的 fd 数量是有上限的（下面会讲），所以“忘记关文件”  
在长期运行的服务里，是一种缓慢的资源泄漏，症状是跑得越久越容易  
报错“打开文件太多”。

----

# dup 复制一份门票

> `dup` 不是复制文件内容，是复制“指向同一份文件状态的门票”——  
> 这解释了 shell 重定向的魔法。

```
dup(3) → 4       # fd 3 和新的 fd 4，指向内核里【同一份】文件状态
```

两个 fd 共享同一份读写位置意味着什么：如果一个进程用 fd 3 读了  
100 字节，紧接着用 fd 4（它的 dup）去读，会从第 101 字节继续读——  
就像两个人轮流看同一本书，谁翻页另一个人看到的也是新的那一页。

这正是 `2>&1` 的实现原理：shell 在启动子进程前，做了大致等价于  
`dup2(1, 2)` 的操作——**把 fd 2 变成 fd 1 当前指向目标的复制品**，  
之后子进程写 fd 2（stderr）和写 fd 1（stdout）就落到了同一个地方  
（比如同一个日志文件）。

Rust 里很少需要手写 `dup`，但标准库确实提供了对应能力（Unix 平台）：

```rust
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

let f = std::fs::File::open("Cargo.toml")?;
let cloned: OwnedFd = f.as_fd().try_clone_to_owned()?; // 语义上等价于 dup
```

日常业务代码基本不需要手动 `dup`——知道它的存在，是为了看懂  
容器启动脚本、shell 重定向、以及“子进程为什么会共享父进程的  
日志文件”这类现象背后的原理。

----

# fd 会被子进程继承

> `fork`/`exec` 出的子进程，默认拿到父进程 fd 表的一份复制——  
> 这是很多“子进程日志混在一起”现象的根源。

- 子进程刚创建时，它的 fd 表是父进程 fd 表的一份**复制**（不是共享，  
  是复制表项，但表项指向的底层文件状态仍是同一份，效果类似 `dup`）；
- 所以父进程 fd 0/1/2 分别是终端输入/输出/错误时，子进程默认也是——  
  这就是为什么你在终端起一个子进程，它的输出照样打在你的终端上，  
  不需要任何额外配置；
- Go 的 `os/exec` 和 Rust 的 `std::process::Command` 默认都遵循这个规则，  
  也都允许显式覆盖：

```rust
use std::process::{Command, Stdio};

Command::new("echo")
    .arg("hi")
    .stdout(Stdio::null()) // 显式覆盖：子进程的 stdout 丢进黑洞
    .status()?;
```

```go
cmd := exec.Command("echo", "hi")
cmd.Stdout = nil // 不设置就默认继承父进程的 stdout
cmd.Run()
```

----

# fd 用尽会报错

> fd 表不是无限大的，每个进程有上限——这是生产事故排查表上  
> 常客“too many open files”的直接原因。

- Linux 上每个进程能打开的 fd 数量有软限制和硬限制，用  
  `ulimit -n` 查看（常见默认值是 1024），生产服务通常会调高；
- 一旦达到上限，`open()`（或者创建 socket）会失败，报错  
  `EMFILE`（Too many open files）——**这不是文件系统的问题，  
  是这一个进程的 fd 计数器爆了**；
- 常见成因：忘记 `close`（连接池没归还连接、循环里反复 `open` 却不关）、  
  或者真的有海量并发连接（下一课会讲这类场景怎么撑住）。

🔬 底层视角：因为 socket 在内核里也是一个 fd（下一节展开），  
`ulimit -n` 同时限制着“最多能开多少个文件”和“最多能开多少个  
网络连接”——这是排查“连接数上不去”问题时，除了应用层配置，  
一定要顺手查一眼系统 fd 上限的原因。

----

# 为 socket 铺路

> 到这里，本课最重要的一句话可以落地了：**socket 在内核里，  
> 就是一个文件描述符。**

回顾前面几节已经出现的所有性质——每个进程一张私有的 fd 表、  
`read`/`write` 的统一接口、`dup` 能复制、`fork` 会继承、  
用尽会报 `EMFILE`——**这些性质对 socket 全部原样成立**，  
因为 socket 走的是同一张表、同一套系统调用接口。

```rust
use std::net::TcpListener;
use std::os::fd::AsRawFd;

let listener = TcpListener::bind("127.0.0.1:0")?;
println!("这个 socket 的 fd 是: {}", listener.as_raw_fd()); // 就是个普通整数
```

这也是为什么 [《socket 就是文件描述符》](../network/socket.md) 那一课，  
可以直接沿用本课的“打开文件表”模型来解释“为什么同一个端口能  
服务成百上千个连接”“为什么关闭连接叫 close 不叫 disconnect”——  
socket 和普通文件在内核数据结构层面走的是**同一条流水线**，  
只是打开方式（`socket()`+`connect()`/`accept()`）不同、  
存的内容（网络缓冲区而不是磁盘数据）不同。

----

# 动手实验

1. 运行 `cargo run -p os-file-fd`，观察程序打印出的各个 fd 数值，  
   验证 stdin/stdout/stderr 恒为 0/1/2、新打开的文件 fd 从 3 起递增；
2. Linux/macOS：程序运行时另开一个终端，执行  
   `ls -l /proc/<pid>/fd/`（Linux）查看该进程此刻打开的所有 fd，  
   对照代码里打开了几个文件；
3. 执行 `./target/debug/os-file-fd > out.log 2>&1`，  
   打开 `out.log` 确认 stdout 和 stderr 的内容确实混在了一起；
4. 执行 `ulimit -n` 查看当前 shell 的 fd 上限，  
   再写一个死循环 `open` 却不 `close` 的小片段，观察它最终报  
   `Too many open files` 时的具体错误。

----

# 三句话带走

1. **fd 只是一个整数索引**，指向本进程私有的“打开文件表”——  
   不是全局唯一的，同一个数字在不同进程里可能指向完全不同的东西。
2. **stdin/stdout/stderr 恒为 0/1/2**，`open`→`read`/`write`→`close`  
   是所有“打开的资源”的统一操作流程；`dup` 能让多个 fd 共享同一份  
   底层状态，是 shell 重定向的原理。
3. **socket 在内核里就是一个 fd**——本课关于 fd 表、继承、  
   `EMFILE` 的所有结论，下一课讲网络连接时全部直接复用。

----

# 附：本课生词表

- **fd（文件描述符 file descriptor）** —— 进程用来标识一个打开资源  
  （文件、socket、管道等）的小整数索引。
- **打开文件表** —— 每个进程私有的一张表，fd 是这张表的索引，  
  表项指向内核里真正的文件状态（读写位置、模式等）。
- **句柄（handle）** —— Windows 上等价于 fd 的概念，思路相同、  
  API 名字不同。
- **stdin/stdout/stderr** —— 每个进程天生拥有的三个流，  
  fd 分别恒为 0/1/2。
- **`dup`** —— 复制一个 fd，让新旧 fd 指向同一份底层文件状态  
  （共享读写位置），是 shell `2>&1` 的实现原理。
- **继承（inherit）** —— 子进程默认拿到父进程 fd 表的一份复制，  
  因此默认共享父进程的 stdin/stdout/stderr。
- **`EMFILE`（Too many open files）** —— 单进程打开的 fd 数量  
  达到上限（`ulimit -n`）时的报错。
- **`/proc/<pid>/fd/`** —— Linux 下查看某进程当前打开的所有 fd  
  及其指向目标的途径，排障利器。
