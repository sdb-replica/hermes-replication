use hermes_replication::{network::NetworkServer, types::NodeRole, ClusterNode};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::time::Duration;

pub struct TestCluster {
    pub node1: Arc<ClusterNode>,
    pub node2: Arc<ClusterNode>,
    pub node3: Arc<ClusterNode>,
}

pub async fn setup_test_cluster(base_port: u16) -> TestCluster {
    let node1 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), base_port),
        NodeRole::Leader,
    ));
    let node2 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), base_port + 1),
        NodeRole::Follower,
    ));
    let node3 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), base_port + 2),
        NodeRole::Follower,
    ));

    // Start servers
    let server1 = NetworkServer::new(Arc::clone(&node1), node1.info.address);
    let server2 = NetworkServer::new(Arc::clone(&node2), node2.info.address);
    let server3 = NetworkServer::new(Arc::clone(&node3), node3.info.address);

    tokio::spawn(async move { server1.start().await });
    tokio::spawn(async move { server2.start().await });
    tokio::spawn(async move { server3.start().await });

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Set up cluster membership
    {
        let mut members1 = node1.members.write().unwrap();
        members1.insert(node1.info.id, node1.info.clone());
        members1.insert(node2.info.id, node2.info.clone());
        members1.insert(node3.info.id, node3.info.clone());

        let mut members2 = node2.members.write().unwrap();
        members2.insert(node1.info.id, node1.info.clone());
        members2.insert(node2.info.id, node2.info.clone());
        members2.insert(node3.info.id, node3.info.clone());

        let mut members3 = node3.members.write().unwrap();
        members3.insert(node1.info.id, node1.info.clone());
        members3.insert(node2.info.id, node2.info.clone());
        members3.insert(node3.info.id, node3.info.clone());
    }

    TestCluster {
        node1,
        node2,
        node3,
    }
}
