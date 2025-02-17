mod test_utils;
use hermes_replication::types::{HermesError, NodeState};
use hermes_replication::{network::NetworkServer, ClusterNode};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use test_utils::setup_test_cluster;

#[tokio::test]
async fn test_write_failures() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cluster = setup_test_cluster(7071).await;

    // Find a key that makes node1 the coordinator
    let mut key = b"test_key".to_vec();
    let mut hash = key.iter().fold(0u64, |acc, &x| acc.wrapping_add(x as u64));
    while {
        let members = cluster.node1.members.read().unwrap();
        let mut active_members: Vec<_> = members.values().collect();
        active_members.sort_by_key(|info| info.id);
        let coordinator_id = active_members[hash as usize % active_members.len()].id;
        coordinator_id != cluster.node1.info.id
    } {
        key.push(b'_');
        hash = key.iter().fold(0u64, |acc, &x| acc.wrapping_add(x as u64));
    }

    // Test: Write to node that's not responsible for the key
    let result = cluster.node2.write(key.clone(), b"value1".to_vec()).await;
    assert!(matches!(result, Err(HermesError::NotResponsible)));

    // Test: Write when all responsible nodes are inactive
    cluster.node1.set_state(NodeState::Inactive);
    let result = cluster.node1.write(key, b"value1".to_vec()).await;
    assert!(matches!(result, Err(HermesError::NoActiveReplicas)));

    Ok(())
}

#[tokio::test]
async fn test_read_failures() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let node1 = Arc::new(ClusterNode::new(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        7081,
    )));

    // Test: Read non-existent key
    let result = node1.read(b"nonexistent".to_vec()).await;
    assert!(matches!(result, Err(HermesError::ValueNotFound(_))));

    // Test: Read from inactive node
    node1.set_state(NodeState::Inactive);
    let result = node1.read(b"key1".to_vec()).await;
    assert!(matches!(result, Err(HermesError::NoActiveReplicas)));

    Ok(())
}

#[tokio::test]
async fn test_network_failures() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create two nodes with node2 having an invalid port to simulate network failure
    let node1 = Arc::new(ClusterNode::new(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        7081,
    )));
    let node2 = Arc::new(ClusterNode::new(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        0,
    )));

    // Set up cluster membership
    {
        let mut members = node1.members.write().unwrap();
        members.clear();
        members.insert(node1.info.id, node1.info.clone());
        members.insert(node2.info.id, node2.info.clone());
    }
    {
        let mut members = node2.members.write().unwrap();
        members.clear();
        members.insert(node1.info.id, node1.info.clone());
        members.insert(node2.info.id, node2.info.clone());
    }

    // Find a key that makes node1 the coordinator
    let mut key = b"test_key".to_vec();
    let mut hash = key.iter().fold(0u64, |acc, &x| acc.wrapping_add(x as u64));
    while {
        let members = node1.members.read().unwrap();
        let mut active_members: Vec<_> = members.values().collect();
        active_members.sort_by_key(|info| info.id);
        let coordinator_id = active_members[hash as usize % active_members.len()].id;
        coordinator_id != node1.info.id
    } {
        key.push(b'_');
        hash = key.iter().fold(0u64, |acc, &x| acc.wrapping_add(x as u64));
    }

    // Write should fail with QuorumFailed when trying to reach other replicas
    let result = node1.write(key, b"value1".to_vec()).await;
    assert!(matches!(result, Err(HermesError::QuorumFailed(_))));

    Ok(())
}

#[tokio::test]
async fn test_quorum_failures() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create two nodes with node2 being unreachable
    let node1 = Arc::new(ClusterNode::new(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        7091,
    )));
    let node2 = Arc::new(ClusterNode::new(SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        7092,
    )));

    // Start only node1
    let server1 = NetworkServer::new(Arc::clone(&node1), node1.info.address);
    tokio::spawn(async move { server1.start().await });
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Set up cluster membership
    {
        let mut members = node1.members.write().unwrap();
        members.clear();
        members.insert(node1.info.id, node1.info.clone());
        members.insert(node2.info.id, node2.info.clone());
    }

    // Find a key that makes node1 the coordinator
    let mut key = b"test_key".to_vec();
    let mut hash = key.iter().fold(0u64, |acc, &x| acc.wrapping_add(x as u64));
    while {
        let members = node1.members.read().unwrap();
        let mut active_members: Vec<_> = members.values().collect();
        active_members.sort_by_key(|info| info.id);
        let coordinator_id = active_members[hash as usize % active_members.len()].id;
        coordinator_id != node1.info.id
    } {
        key.push(b'_');
        hash = key.iter().fold(0u64, |acc, &x| acc.wrapping_add(x as u64));
    }

    // Write should fail because node2 is unreachable
    let result = node1.write(key, b"value1".to_vec()).await;
    assert!(matches!(result, Err(HermesError::QuorumFailed(_))));

    Ok(())
}
