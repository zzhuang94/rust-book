use std::time::Duration;

use tokio::net::UdpSocket;

#[tokio::test]
async fn udp_端口不回包时必须超时退出() {
    // 绑定一个真实 UDP 端口，但故意不调用 recv_from 和 send_to，
    // 稳定模拟“请求已发出，却永远没有响应”。
    let silent_peer = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let peer_addr = silent_peer.local_addr().unwrap();
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    client.send_to(b"ping", peer_addr).await.unwrap();

    let mut buf = [0u8; 32];
    let result = tokio::time::timeout(
        Duration::from_millis(30),
        client.recv_from(&mut buf),
    )
    .await;

    assert!(result.is_err(), "没有回包时，客户端必须在预算内结束等待");
}
