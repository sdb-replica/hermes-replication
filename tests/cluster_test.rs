use hermes_replication::{ClusterMessage, ClusterNode, NodeRole, network::{NetworkServer, NetworkClient}};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio;

#[tokio::test]
async fn test_three_node_cluster() -> Result<(), Box<dyn std::error::Error>> {
    // Create 3 nodes with different ports
    let node1 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081),
        NodeRole::Leader
    ));
    let node2 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8082),
        NodeRole::Follower
    ));
    let node3 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8083),
        NodeRole::Follower
    ));

    // Start servers
    let server1 = NetworkServer::new(Arc::clone(&node1), node1.info.address);
    let server2 = NetworkServer::new(Arc::clone(&node2), node2.info.address);
    let server3 = NetworkServer::new(Arc::clone(&node3), node3.info.address);

    // Start servers in separate tasks
    tokio::spawn(async move { server1.start().await });
    tokio::spawn(async move { server2.start().await });
    tokio::spawn(async move { server3.start().await });

    // Give servers time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Create client and connect to node1
    let mut client = NetworkClient::connect(node1.info.address).await?;

    // Test join request
    let join_msg = ClusterMessage::JoinRequest(node2.info.clone());
    let response = client.send(join_msg).await?;

    assert!(matches!(
        response,
        Some(ClusterMessage::JoinResponse(true))
    ));

    // Test heartbeat
    let heartbeat = ClusterMessage::HeartBeat;
    let response = client.send(heartbeat).await?;

    assert!(matches!(
        response,
        Some(ClusterMessage::HeartBeatAck)
    ));

    Ok(())
} 