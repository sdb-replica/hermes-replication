mod test_utils;
use hermes_replication::types::{HermesError, NodeRole, NodeState};
use hermes_replication::{network::NetworkServer, ClusterNode};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use test_utils::setup_test_cluster;

#[tokio::test]
async fn test_write_failures() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cluster = setup_test_cluster(7071).await;

    // Test 1: Write to node that's not responsible for the key
    let result = cluster
        .node2
        .write("key1".to_string(), b"value1".to_vec())
        .await;
    assert!(matches!(result, Err(HermesError::NotResponsible)));

    // Test 2: Write when all responsible nodes are inactive
    cluster.node1.set_state(NodeState::Inactive); // Set leader to inactive
    let result = cluster
        .node1
        .write("key1".to_string(), b"value1".to_vec())
        .await;
    assert!(matches!(result, Err(HermesError::NoActiveReplicas)));

    Ok(())
}

#[tokio::test]
async fn test_read_failures() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let node1 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7081),
        NodeRole::Leader,
    ));

    // Test 1: Read non-existent key
    let result = node1.read("nonexistent".to_string()).await;
    assert!(matches!(result, Err(HermesError::ValueNotFound(_))));

    // Test 2: Read from inactive node
    node1.set_state(NodeState::Inactive);
    let result = node1.read("key1".to_string()).await;
    assert!(matches!(result, Err(HermesError::NoActiveReplicas)));

    Ok(())
}

#[tokio::test]
async fn test_network_failures() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create leader node with valid port
    let node1 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7081),
        NodeRole::Leader,
    ));

    // Create follower node with invalid port to simulate network failure
    let node2 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0),
        NodeRole::Follower,
    ));

    // Add follower to leader's member list
    node1
        .members
        .write()
        .unwrap()
        .insert(node2.info.id, node2.info.clone());

    // Test write - should fail with NetworkError when trying to reach follower
    let result = node1.write("key1".to_string(), b"value1".to_vec()).await;
    assert!(matches!(result, Err(HermesError::NetworkError(_))));

    // Test read - should fail with NetworkError when trying to reach follower
    let result = node1.read("key1".to_string()).await;
    assert!(matches!(result, Err(HermesError::NetworkError(_))));

    Ok(())
}

#[tokio::test]
async fn test_quorum_failures() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let node1 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7091),
        NodeRole::Leader,
    ));
    let node2 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 7092),
        NodeRole::Follower,
    ));

    // Start only node1
    let server1 = NetworkServer::new(Arc::clone(&node1), node1.info.address);
    tokio::spawn(async move { server1.start().await });
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Add node2 to node1's members and ensure it's active
    {
        let mut members = node1.members.write().unwrap();
        let mut node2_info = node2.info.clone();
        node2_info.state = NodeState::Active; // Ensure node2 is active
        members.insert(node2.info.id, node2_info);
    }

    // Write should fail because node2 is unreachable
    let result = node1.write("key1".to_string(), b"value1".to_vec()).await;
    assert!(matches!(result, Err(HermesError::QuorumFailed(_))));

    Ok(())
}
