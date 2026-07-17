use labkit::logln;
use redis::cluster::ClusterClient;
use redis::AsyncCommands;

/// 从环境变量读取若干 seed node。
///
/// seed 只是“敲门地址”：客户端连上任意一个可用节点后，会读取完整槽位拓扑。
/// 多写几个 seed，可以避免程序启动时恰好碰到唯一节点不可用。
fn seed_nodes() -> Vec<String> {
    std::env::var("REDIS_CLUSTER_NODES")
        .unwrap_or_else(|_| {
            "redis://127.0.0.1:7000,redis://127.0.0.1:7001,redis://127.0.0.1:7002".into()
        })
        .split(',')
        .map(str::trim)
        .filter(|node| !node.is_empty())
        .map(str::to_owned)
        .collect()
}

#[tokio::main]
async fn main() -> redis::RedisResult<()> {
    let seeds = seed_nodes();
    logln!("准备通过 {} 个 seed node 发现集群", seeds.len());

    // new 只检查地址；真正连接和读取槽位表发生在下一行。
    let client = ClusterClient::new(seeds)?;
    let mut connection = client.get_async_connection().await?;

    // 普通单 key 命令由客户端计算槽位，并自动路由到正确节点。
    let visits: i64 = connection.incr("tutorial:visits", 1).await?;
    logln!("tutorial:visits = {visits}");

    // 花括号里的 42 是 hash tag。两个 key 会进入同一个槽位，
    // 因而可以放进同一条 MGET、事务或 Lua 脚本。
    let profile_key = "tutorial:user:{42}:profile";
    let settings_key = "tutorial:user:{42}:settings";
    let _: () = connection.set(profile_key, "basic").await?;
    let _: () = connection.set(settings_key, "dark").await?;

    let values: Vec<Option<String>> = redis::cmd("MGET")
        .arg([profile_key, settings_key])
        .query_async(&mut connection)
        .await?;
    logln!("同槽批量读取：{values:?}");

    Ok(())
}

