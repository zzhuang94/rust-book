//! 编译期把 proto/helloworld.proto 翻译成 Rust 代码（客户端 stub + 服务端 trait + 消息结构体）。
//!
//! tonic 0.13 起，"读 .proto 生成代码" 这部分从 tonic-build 拆到了专门的
//! tonic-prost-build crate（tonic-build 本身只留下不依赖 prost 的通用 codegen 基础设施）。
//! 生成的代码由 src/main.rs 里的 `tonic::include_proto!("helloworld")` 引入。

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::compile_protos("proto/helloworld.proto")?;
    Ok(())
}
