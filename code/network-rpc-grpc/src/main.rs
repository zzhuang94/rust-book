//! gRPC 最小 demo：tonic + prost 实现 Greeter.SayHello，同进程起「服务端 + 客户端」。
//!
//! proto 定义见 proto/helloworld.proto；build.rs 里的 tonic-prost-build 在编译期把它
//! 翻译成下面 `helloworld` 模块里 include! 进来的 Rust 代码（消息结构体 + client/server stub）。
//!
//! 只用明文 h2：tonic 的默认特性（router + transport + codegen）本身就不包含任何
//! `tls-*` 特性，天然走不加密的 HTTP/2，不用额外关闭什么。
//!
//! tonic 的 `Server::serve` 要一个具体地址，不像 `TcpListener::bind` 那样能传端口 0
//! 再回读实际端口，所以这里用固定端口 50051；绑定前先探测一下端口是否被占用，
//! 占用就打印提示后体面退出，不 panic。
//!
//! 运行（在 code/ 下）：cargo run -p network-rpc-grpc

use std::net::SocketAddr;
use std::time::Duration;

use labkit::logln;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

pub mod helloworld {
    tonic::include_proto!("helloworld");
}

use helloworld::greeter_client::GreeterClient;
use helloworld::greeter_server::{Greeter, GreeterServer};
use helloworld::{HelloReply, HelloRequest};

const ADDR: &str = "127.0.0.1:50051";

#[derive(Default)]
struct MyGreeter;

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        let name = request.into_inner().name;
        logln!("服务端收到请求：name={name}");
        Ok(Response::new(HelloReply {
            message: format!("Hello, {name}!"),
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr: SocketAddr = ADDR.parse()?;

    // 先探测一次端口是否可用：绑上就立刻释放，真正的监听交给下面的 Server::serve。
    // 这里存在极小的 TOCTOU 竞态（探测和真正绑定之间端口理论上可能被别的进程抢走），
    // 教学 demo 不追求绝对严谨，重点是演示「捕获错误、不 panic、给出人话提示」这个套路。
    if let Err(err) = std::net::TcpListener::bind(addr) {
        logln!("绑定 {ADDR} 失败（{err}），端口可能已被占用，程序退出。");
        return Ok(());
    }

    let server_task = tokio::spawn(async move {
        logln!("gRPC 服务监听 {ADDR}");
        Server::builder()
            .add_service(GreeterServer::new(MyGreeter))
            .serve(addr)
            .await
    });

    // 服务端协程刚起步时端口可能还没真正 listen 好，客户端连接前稍等一下更稳妥。
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = GreeterClient::connect(format!("http://{ADDR}")).await?;
    let response = client
        .say_hello(HelloRequest {
            name: "tonic".into(),
        })
        .await?;
    logln!("客户端收到回复：{}", response.into_inner().message);

    // demo 到此为止：服务端任务留给进程退出时一并回收，不用等它。
    server_task.abort();
    Ok(())
}
