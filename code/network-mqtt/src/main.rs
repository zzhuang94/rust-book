//! MQTT 最小 demo：用 rumqttc 对本地 Mosquitto broker 做一轮 pub/sub。
//!
//! MQTT 是「发布/订阅」模型：客户端不直接互相连接，而是都连到一个 broker，
//! 订阅某个 topic 的客户端会收到别人发到这个 topic 上的所有消息。
//!
//! 先起一个本地 broker（官方 eclipse-mosquitto 镜像，不用公司内部镜像）：
//!   docker run --rm -it -p 1883:1883 eclipse-mosquitto:2 \
//!       sh -c "printf 'listener 1883\\nallow_anonymous true\\n' > /tmp/mq.conf && mosquitto -c /tmp/mq.conf"
//! （eclipse-mosquitto 官方镜像默认要求认证/走配置文件，这里现场写一个只监听
//!  1883、允许匿名连接的最小配置，纯本地教学用；生产环境务必配认证 + TLS。）
//!
//! 运行（在 code/ 下）：cargo run -p network-mqtt
//! 连不上 broker 时不会 panic：打印清晰提示后直接正常退出（Ok(())）。

use std::time::Duration;

use labkit::logln;
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS};
use tokio::net::TcpStream;
use tokio::time::timeout;

const BROKER_HOST: &str = "127.0.0.1";
const BROKER_PORT: u16 = 1883;
const TOPIC: &str = "tutorial/ping";

/// 用一次裸 TCP 连接探测 broker 是否在跑。
///
/// 这样连不上时可以在这里就体面退出，而不是让 rumqttc 的 EventLoop
/// 在背后没完没了地重连、最后卡住看不出原因。
async fn broker_reachable() -> bool {
    let addr = format!("{BROKER_HOST}:{BROKER_PORT}");
    matches!(
        timeout(Duration::from_secs(1), TcpStream::connect(addr)).await,
        Ok(Ok(_))
    )
}

/// 订阅一次、收到 SubAck 后发一条消息，再等自己那条消息被推回来。
///
/// rumqttc 是「客户端句柄 + EventLoop」两件套：AsyncClient 只负责把指令丢进
/// 内部队列，真正的网络 IO（连接、心跳、收发包）全靠不断 poll EventLoop 驱动。
async fn run_pubsub(client: AsyncClient, mut eventloop: EventLoop) -> anyhow::Result<()> {
    client.subscribe(TOPIC, QoS::AtMostOnce).await?;
    logln!("已发出订阅请求：{TOPIC}");

    loop {
        match eventloop.poll().await? {
            Event::Incoming(Packet::SubAck(_)) => {
                logln!("订阅已确认，发布一条消息");
                client
                    .publish(TOPIC, QoS::AtMostOnce, false, b"hello mqtt".to_vec())
                    .await?;
            }
            Event::Incoming(Packet::Publish(msg)) => {
                logln!(
                    "收到消息：topic={} payload={:?}",
                    msg.topic,
                    String::from_utf8_lossy(&msg.payload)
                );
                return Ok(());
            }
            // ConnAck/PingResp/PubAck 等其他事件本课不关心。
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if !broker_reachable().await {
        logln!("连不上 MQTT broker（{BROKER_HOST}:{BROKER_PORT}）。");
        logln!("请先启动一个本地 Mosquitto，例如：");
        logln!(
            "  docker run --rm -it -p 1883:1883 eclipse-mosquitto:2 \\"
        );
        logln!(
            "      sh -c \"printf 'listener 1883\\nallow_anonymous true\\n' > /tmp/mq.conf && mosquitto -c /tmp/mq.conf\""
        );
        logln!("跳过本次 pub/sub 演示，程序正常退出。");
        return Ok(());
    }

    let mut options = MqttOptions::new("network-mqtt-demo", BROKER_HOST, BROKER_PORT);
    options.set_keep_alive(Duration::from_secs(5));

    // 第二个参数是客户端到 EventLoop 之间指令队列的容量，教学用小一点足够。
    let (client, eventloop) = AsyncClient::new(options, 10);

    match timeout(Duration::from_secs(5), run_pubsub(client, eventloop)).await {
        Ok(result) => result,
        Err(_) => {
            logln!("5 秒内没跑完一轮 pub/sub，可能是 broker 状态异常，程序退出。");
            Ok(())
        }
    }
}
