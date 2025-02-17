use hermes_replication::network::NetworkClient;
use hermes_replication::types::ClusterMessage;
use hermes_replication::{network::NetworkServer, ClusterNode};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::time::Duration;

#[allow(dead_code)]
pub struct TestCluster {
    pub node1: Arc<ClusterNode>,
    pub node2: Arc<ClusterNode>,
    pub node3: Arc<ClusterNode>,
}

pub async fn setup_test_cluster(base_port: u16) -> TestCluster {
    let node1 = Arc::new(ClusterNode::new(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        base_port,
    )));
    let node2 = Arc::new(ClusterNode::new(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        base_port + 1,
    )));
    let node3 = Arc::new(ClusterNode::new(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        base_port + 2,
    )));

    // Start servers
    let server1 = NetworkServer::new(Arc::clone(&node1), node1.info.address);
    let server2 = NetworkServer::new(Arc::clone(&node2), node2.info.address);
    let server3 = NetworkServer::new(Arc::clone(&node3), node3.info.address);

    tokio::spawn(async move { server1.start().await });
    tokio::spawn(async move { server2.start().await });
    tokio::spawn(async move { server3.start().await });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Join nodes to cluster
    let mut client = NetworkClient::connect(node1.info.address).await.unwrap();
    client
        .send(ClusterMessage::JoinRequest(node2.info.clone()))
        .await
        .unwrap();
    client
        .send(ClusterMessage::JoinRequest(node3.info.clone()))
        .await
        .unwrap();

    // Ensure all nodes have the same membership list
    let mut client2 = NetworkClient::connect(node2.info.address).await.unwrap();
    let mut client3 = NetworkClient::connect(node3.info.address).await.unwrap();

    client2
        .send(ClusterMessage::JoinRequest(node1.info.clone()))
        .await
        .unwrap();
    client2
        .send(ClusterMessage::JoinRequest(node3.info.clone()))
        .await
        .unwrap();

    client3
        .send(ClusterMessage::JoinRequest(node1.info.clone()))
        .await
        .unwrap();
    client3
        .send(ClusterMessage::JoinRequest(node2.info.clone()))
        .await
        .unwrap();

    TestCluster {
        node1,
        node2,
        node3,
    }
}

#[allow(dead_code)]
pub fn get_coordinator<'a>(key: &[u8], nodes: &'a TestCluster) -> &'a Arc<ClusterNode> {
    let hash = key.iter().fold(0u64, |acc, &x| acc.wrapping_add(x as u64));
    let members = nodes.node1.members.read().unwrap();
    let mut active_members: Vec<_> = members.values().collect();
    active_members.sort_by_key(|info| info.id);
    let coordinator_id = active_members[hash as usize % active_members.len()].id;

    if coordinator_id == nodes.node1.info.id {
        &nodes.node1
    } else if coordinator_id == nodes.node2.info.id {
        &nodes.node2
    } else {
        &nodes.node3
    }
}
