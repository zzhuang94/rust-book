# 把读做成无锁

> 代码：`code/http-arcswap/`　运行：`cargo run -p http-arcswap`（功能、  
> 接口与 [《从零手写 HTTP》](http-from-scratch.md) 完全相同，只换了状态层的实现，方便直接对比）

[《从零手写 HTTP》](http-from-scratch.md) 用 `Arc<RwLock<Snapshot>>`，  
读要拿读锁。这一课换成 **ArcSwap**，让读路径 **完全无锁**。

----

# RwLock 读的成本

> 读锁允许多读并发，但并非零成本。先算笔账。

1. **读锁要记账**：每个读者进出临界区都要 **原子地** 改「当前读者数」计数器。高 QPS 下成千上万读者争同一个计数器，  
   cache line 在多核间来回弹跳（cache line bouncing），开销可观；
2. **读写仍会互相等待**：写者拿写锁要等 **所有** 读者退出；写锁持有期间读者全被挡。写虽极短，极端并发下这个互斥窗口仍会制造尾延迟毛刺；
3. **本项目数据形态特别适合更激进的方案**：「整份数据定期替换」+「读远多于写」——正是无锁方案的主场。

----

# RCU 读只取指针

> **ArcSwap** 实现的是 **RCU（Read-Copy-Update）** 模式，核心一句话：把整份数据放进一个「可原子交换的指针」。  
> **读 = 原子读一个指针；写 = 原子换一个指针。**

```rust
struct Inner {
    data: ArcSwap<Snapshot>,   // 装着 Arc<Snapshot> 的原子槽位
    reads: AtomicU64,
}
```

**读**（无锁）：

```rust
pub fn load(&self) -> Arc<Snapshot> {
    self.inner.data.load_full()  // 原子取出当前 Arc（计数+1）
}                                 // 无锁、O(1)、绝不阻塞
```

**写**（原子替换）：

```rust
pub fn store(&self, snap: Snapshot) {
    self.inner.data.store(Arc::new(snap));  // 一次原子换指针
}
```

时间线看「读写为什么不打架」：

```
        t0            t1(写者 store)          t2
读者R1:  load()→版本A  [------ 继续安全地用 A ------]  用完 drop A
写者W:                 store(B)：指针 A→B（一瞬间）
读者R2:                            load()→版本B
                       ↑ R2 拿到 B，R1 手里还是 A，互不干扰
```

流程拆解：写者在旁边 **不慌不忙** 构造好新 Snapshot；store 的一瞬间指针从 A 换成 B；正在读 A 的读者毫发无伤——手里的 `Arc<Snapshot>` 还指着 A；  
这些读者用完、A 的计数归零，A 自动回收； **读者与写者从不互相阻塞**，读者之间更不用说。

> **深入：旧版本何时安全回收？** 这是 RCU 的精髓，也是 C 语言手写 RCU 最难的部分（要处理「宽限期 grace period」）。  
> Rust 用 `Arc` 引用计数优雅解决：还有读者持有 → 计数不归零 → 不释放；最后一个读者一走 → 归零 → 自动 drop。  
> 你完全不用操心。

对照 Go：

```go
var data atomic.Pointer[Snapshot]
snap := data.Load()    // 读：无锁
data.Store(&fresh)     // 写：原子替换
```

Go 最接近的是 `atomic.Pointer[T]` 手写 RCU；旧 `*Snapshot` 何时回收？靠 **GC**。  
ArcSwap = 「原子指针 + 引用计数的安全回收」打包好，且回收是 **确定性** 的（归零立即释放，不等 GC）。

----

# 为何外包一层 Arc

> 一个 Rust 特有细节：`ArcSwap<T>` 本身 **不是 `Clone`**，而 axum 的 State 要求 Clone。  
> 解法是一个高频模式，记住它。

```rust
struct Inner {                 // 所有共享字段收进一个（不 pub 的）Inner
    data: ArcSwap<Snapshot>,
    reads: AtomicU64,
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,         // clone AppState = clone Arc = 计数+1
}
```

AppState 随便 clone（每请求一份），全都共享同一个 Inner。对照 Go：直接传 `*AppState` 指针，  
GC 兜底；Rust 用 `Arc<Inner>` 显式表达「共享 + 计数回收」。

----

# 响应不用克隆

> 顺带的好处：读接口连结构体都不用克隆。

- [《从零手写 HTTP》](http-from-scratch.md) 读接口：`state.snapshot()` 里 `.clone()` 了整个 Snapshot（含 Vec，  
  深拷贝）；
- 本课读接口：直接把 `Arc<Snapshot>` 交给序列化：

```rust
pub async fn get_data(State(state): State<AppState>) -> Json<Arc<Snapshot>> {
    Json(state.load())   // 没克隆结构体，只是计数 +1
}
```

能序列化 `Arc<T>` 是因为工作区给 serde 开了 `rc` 特性（根 Cargo.toml `features = ["derive", "rc"]`）。

> **说句实话（别被「零拷贝」忽悠）**：HTTP 响应最终还是要序列化成 JSON 字节，这步开销跑不掉；ArcSwap 真正省的是共享状态读取时的 **锁记账**，  
> 以及那次 **Snapshot 深拷贝**；「读出来就地计算、不发网络」的内部读者收益最直接。理解清楚收益边界，  
> 才不会盲目上无锁。

----

# RwLock 还是 ArcSwap

> 两者怎么选，一张表 + 一条经验法则。

| | `Arc<RwLock<T>>` | `ArcSwap<T>` |
| --- | --- | --- |
| 读 | 拿读锁（原子记账，有争用） | 无锁，O(1) 取指针 |
| 写 | 写锁独占，等读者退出 | 原子换指针，不等读者 |
| 读写互相阻塞 | 会（短暂） | 完全不会 |
| 局部更新（改一个字段） | ✅ 可原地改 | ❌ 得整份重造 |
| 高并发尾延迟 | 偶有毛刺 | 更平稳 |
| 心智复杂度 | 低（像 RWMutex） | 略高（RCU、Inner 包裹） |
| 依赖 | 标准库 | 第三方 `arc-swap` |

经验法则：入门 / 一般读写 / 需要局部更新 → `Arc<RwLock<T>>`；「整份定期替换」+ 读 QPS 极高、  
在意尾延迟 → ArcSwap（本课，正是你需求的形态）；写很频繁、或写要基于旧值增量 → RwLock 更合适。

----

# 动手实验

1. **对比压测**：RwLock 版和 ArcSwap 版各压一遍 `ab -n 50000 -c 500 http://127.0.0.1:7080/data`，  
   对比 Requests/sec 和 99% 分位。核越多、并发越高，ArcSwap 优势通常越明显（本地环回噪声大，  
   看趋势）；
2. **观察引用计数回收**：给 Snapshot 加 `impl Drop for Snapshot { fn drop(&mut self){ println!("drop v{}", self.version) } }`，  
   看旧版本是「有读者时」还是「无读者时」打印 drop。跑完删掉；
3. **体会局部更新的别扭**：想「只把 items[0] +1」——RwLock 是 `guard.items[0] += 1`；  
   ArcSwap 得 load → clone 整份 → 改 → store。亲手写一遍。

----

# 三句话带走

1. **ArcSwap = RCU**：读无锁取指针、写原子换指针，读写永不互阻；旧版本靠 Arc 计数 **确定性** 回收（不等 GC）。
2. 它天然契合「整份定期替换 + 海量并发读」；省的是锁记账和深拷贝，序列化开销仍在。
3. 代价：心智稍复杂（Inner + 外包 Arc）、引第三方 crate、不适合高频写/局部更新。`Arc<RwLock>` 够用时别硬上，  
   **按需** 升级。

----

# 附：本课生词表

> 通用语法见 [《Rust 语法底座》](../start/syntax-primer.md)；Arc/原子量见 [《共享状态：Arc / RwLock》](../async/shared-state.md)，  
> State/serde 见 [《从零手写 HTTP》](http-from-scratch.md)。

- **`arc_swap::ArcSwap<T>`** ——第三方 crate `arc-swap` 的「可原子替换的 Arc 槽位」；  
  想成一个特殊变量：永远装着一个 `Arc<T>`，换「装的是哪个 Arc」是原子操作；注意 crate 名带连字符 `arc-swap`，  
  代码里模块名变下划线 `arc_swap`。
- **`ArcSwap::new(Arc::new(v))`** ——初始化：数据装进 Arc，再放入槽位；便捷写法 `ArcSwap::from_pointee(v)`。
- **`.load_full()`** ——无锁读：原子取出当前 `Arc<T>`（计数+1）返回；拿到完整 Arc，  
  随便用多久，不影响写者替换槽位；另有更快的 `.load()`（返回临时守卫），初学统一 load_full。
- **`.store(Arc::new(fresh))`** ——无锁写：原子换槽位里的 Arc；旧 Arc 计数 -1，  
  最后一个使用者 drop 后旧数据自动释放。
- **`Inner` + 外包 `Arc`（组织手法）** ——因为 ArcSwap 不是 Clone、State 要 Clone；  
  共享字段收进（不 pub 的）Inner，`AppState { inner: Arc<Inner> }` + derive Clone；  
  Rust 服务端高频模式。
- **`Json<Arc<Snapshot>>`** ——serde 默认不给 `Arc<T>` 实现 Serialize；  
  工作区开 serde `rc` 特性才解锁；收益：handler 连 Snapshot 都不 clone，直接交给序列化器。
- **`impl Drop for Snapshot`** ——`Drop` trait = 析构钩子：值被释放那一刻自动调用；  
  ≈ Go 的 finalizer，但 **确定性** 触发（作用域结束/计数归零），不是等 GC。
