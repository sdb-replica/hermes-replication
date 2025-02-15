mod test_utils;
use hermes_replication::{
    network::{NetworkClient, NetworkServer},
    ClusterMessage, ClusterNode,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use test_utils::setup_test_cluster;

#[tokio::test]
async fn test_three_node_cluster() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create 3 nodes with different ports
    let node1 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081),
    ));
    let node2 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8082),
    ));
    let node3 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8083),
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

    assert!(matches!(response, Some(ClusterMessage::JoinResponse(members)) if !members.is_empty()));

    // Test heartbeat
    let heartbeat = ClusterMessage::HeartBeat;
    let response = client.send(heartbeat).await?;

    assert!(matches!(response, Some(ClusterMessage::HeartBeatAck)));

    Ok(())
}

#[tokio::test]
async fn test_write_and_read() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create nodes
    let node1 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8091)
    ));
    let node2 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8092)
    ));
    let node3 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8093)
    ));

    // Start servers
    let server1 = NetworkServer::new(Arc::clone(&node1), node1.info.address);
    let server2 = NetworkServer::new(Arc::clone(&node2), node2.info.address);
    let server3 = NetworkServer::new(Arc::clone(&node3), node3.info.address);

    tokio::spawn(async move { server1.start().await });
    tokio::spawn(async move { server2.start().await });
    tokio::spawn(async move { server3.start().await });

    // Give servers time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Join nodes to cluster
    let mut client = NetworkClient::connect(node1.info.address).await?;
    client
        .send(ClusterMessage::JoinRequest(node2.info.clone()))
        .await?;
    client
        .send(ClusterMessage::JoinRequest(node3.info.clone()))
        .await?;

    // Find coordinator for test key
    let test_key = "test_key".to_string();
    let hash = test_key.as_bytes().iter().fold(0u64, |acc, &x| acc.wrapping_add(x as u64));
    let members = node1.members.read().unwrap();
    let mut active_members: Vec<_> = members.values().collect();
    active_members.sort_by_key(|info| info.id);
    let coordinator_id = active_members[hash as usize % active_members.len()].id;

    // Write through coordinator
    let coordinator = if coordinator_id == node1.info.id {
        &node1
    } else if coordinator_id == node2.info.id {
        &node2
    } else {
        &node3
    };

    coordinator
        .write(test_key.clone(), b"test_value".to_vec())
        .await
        .map_err(|e| format!("Write failed: {}", e))?;

    // Read data back from each node
    let value1 = node1
        .read(test_key.clone())
        .await
        .map_err(|e| format!("Read from node1 failed: {}", e))?;
    let value2 = node2
        .read(test_key.clone())
        .await
        .map_err(|e| format!("Read from node2 failed: {}", e))?;
    let value3 = node3
        .read(test_key.clone())
        .await
        .map_err(|e| format!("Read from node3 failed: {}", e))?;

    // Assert all nodes have the same value
    assert_eq!(value1, b"test_value");
    assert_eq!(value2, b"test_value");
    assert_eq!(value3, b"test_value");

    Ok(())
}

#[tokio::test]
async fn test_hermes_protocol() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cluster = setup_test_cluster(9091).await;

    // Find coordinator for key1
    let key1 = "key1".to_string();
    let hash = key1.as_bytes().iter().fold(0u64, |acc, &x| acc.wrapping_add(x as u64));
    let members = cluster.node1.members.read().unwrap();
    let mut active_members: Vec<_> = members.values().collect();
    active_members.sort_by_key(|info| info.id);
    let coordinator_id = active_members[hash as usize % active_members.len()].id;

    // Write through coordinator
    let coordinator = if coordinator_id == cluster.node1.info.id {
        &cluster.node1
    } else if coordinator_id == cluster.node2.info.id {
        &cluster.node2
    } else {
        &cluster.node3
    };

    // Write initial value
    coordinator.write(key1.clone(), b"value1".to_vec()).await?;

    // Read from all nodes should succeed
    let val1 = cluster.node1.read(key1.clone()).await?;
    let val2 = cluster.node2.read(key1.clone()).await?;
    let val3 = cluster.node3.read(key1.clone()).await?;
    assert_eq!(val1, b"value1");
    assert_eq!(val2, b"value1");
    assert_eq!(val3, b"value1");

    // Update value through same coordinator
    coordinator.write(key1.clone(), b"value1_updated".to_vec()).await?;

    // Read updated value from all nodes
    let val1 = cluster.node1.read(key1.clone()).await?;
    let val2 = cluster.node2.read(key1.clone()).await?;
    let val3 = cluster.node3.read(key1.clone()).await?;
    assert_eq!(val1, b"value1_updated");
    assert_eq!(val2, b"value1_updated");
    assert_eq!(val3, b"value1_updated");

    // Write new key
    let key2 = "key2".to_string();
    let hash = key2.as_bytes().iter().fold(0u64, |acc, &x| acc.wrapping_add(x as u64));
    let members = cluster.node1.members.read().unwrap();
    let mut active_members: Vec<_> = members.values().collect();
    active_members.sort_by_key(|info| info.id);
    let coordinator_id = active_members[hash as usize % active_members.len()].id;

    // Write through coordinator
    let coordinator = if coordinator_id == cluster.node1.info.id {
        &cluster.node1
    } else if coordinator_id == cluster.node2.info.id {
        &cluster.node2
    } else {
        &cluster.node3
    };

    coordinator.write(key2.clone(), b"value2".to_vec()).await?;

    // Read new key from all nodes
    let val1 = cluster.node1.read(key2.clone()).await?;
    let val2 = cluster.node2.read(key2.clone()).await?;
    let val3 = cluster.node3.read(key2.clone()).await?;
    assert_eq!(val1, b"value2");
    assert_eq!(val2, b"value2");
    assert_eq!(val3, b"value2");

    Ok(())
}
