use hermes_replication::types::NodeState;
use hermes_replication::{
    network::{NetworkClient, NetworkServer},
    ClusterMessage, ClusterNode, NodeRole,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

#[tokio::test]
async fn test_three_node_cluster() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create 3 nodes with different ports
    let node1 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081),
        NodeRole::Leader,
    ));
    let node2 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8082),
        NodeRole::Follower,
    ));
    let node3 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8083),
        NodeRole::Follower,
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
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8091),
        NodeRole::Leader,
    ));
    let node2 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8092),
        NodeRole::Follower,
    ));
    let node3 = Arc::new(ClusterNode::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8093),
        NodeRole::Follower,
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

    // Write data through node1
    let test_key = "test_key".to_string();
    let test_value = b"test_value".to_vec();
    node1
        .write(test_key.clone(), test_value.clone())
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
    assert_eq!(value1, test_value);
    assert_eq!(value2, test_value);
    assert_eq!(value3, test_value);

    Ok(())
}

#[tokio::test]
async fn test_hermes_protocol() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let test = async {
        // Create a 3-node cluster
        let node1 = Arc::new(ClusterNode::new(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9091),
            NodeRole::Leader,
        ));
        let node2 = Arc::new(ClusterNode::new(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9092),
            NodeRole::Follower,
        ));
        let node3 = Arc::new(ClusterNode::new(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 9093),
            NodeRole::Follower,
        ));

        // Start servers
        let server1 = NetworkServer::new(Arc::clone(&node1), node1.info.address);
        let server2 = NetworkServer::new(Arc::clone(&node2), node2.info.address);
        let server3 = NetworkServer::new(Arc::clone(&node3), node3.info.address);

        tokio::spawn(async move { server1.start().await });
        tokio::spawn(async move { server2.start().await });
        tokio::spawn(async move { server3.start().await });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Join nodes to cluster and wait for responses
        let mut client = NetworkClient::connect(node1.info.address).await?;

        // Join node2 and update its members
        let response = client
            .send(ClusterMessage::JoinRequest(node2.info.clone()))
            .await?;
        if let Some(ClusterMessage::JoinResponse(members)) = response {
            let mut members_guard = node2.members.write().unwrap();
            for member in members {
                members_guard.insert(member.id, member);
            }
        }

        // Join node3 and update both node2 and node3's members
        let response = client
            .send(ClusterMessage::JoinRequest(node3.info.clone()))
            .await?;
        if let Some(ClusterMessage::JoinResponse(members)) = response {
            // Update node2's members
            let mut members_guard = node2.members.write().unwrap();
            for member in members.clone() {
                members_guard.insert(member.id, member);
            }

            // Update node3's members
            let mut members_guard = node3.members.write().unwrap();
            for member in members {
                members_guard.insert(member.id, member);
            }
        }

        // Give time for membership to propagate
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify all nodes have all members
        assert_eq!(node1.members.read().unwrap().len(), 3);
        assert_eq!(node2.members.read().unwrap().len(), 3);
        assert_eq!(node3.members.read().unwrap().len(), 3);

        // Test 1: Basic write and read
        let key1 = "key1".to_string();
        let value1 = b"value1".to_vec();
        node1
            .write(key1.clone(), value1.clone())
            .await
            .map_err(|e| format!("Initial write failed: {}", e))?;

        // Verify all nodes have the same value
        let read1 = node1.read(key1.clone()).await?;
        let read2 = node2.read(key1.clone()).await?;
        let read3 = node3.read(key1.clone()).await?;
        assert_eq!(read1, value1);
        assert_eq!(read2, value1);
        assert_eq!(read3, value1);

        // Test 2: Update existing value
        let value1_updated = b"value1_updated".to_vec();
        node1
            .write(key1.clone(), value1_updated.clone())
            .await
            .map_err(|e| format!("Update write failed: {}", e))?;

        // Verify update propagated
        let read1 = node1.read(key1.clone()).await?;
        let read2 = node2.read(key1.clone()).await?;
        let read3 = node3.read(key1.clone()).await?;
        assert_eq!(read1, value1_updated);
        assert_eq!(read2, value1_updated);
        assert_eq!(read3, value1_updated);

        // Test 3: Write through follower
        let key2 = "key2".to_string();
        let value2 = b"value2".to_vec();
        node2
            .write(key2.clone(), value2.clone())
            .await
            .map_err(|e| format!("Follower write failed: {}", e))?;

        // Verify write through follower worked
        let read1 = node1.read(key2.clone()).await?;
        let read2 = node2.read(key2.clone()).await?;
        let read3 = node3.read(key2.clone()).await?;
        assert_eq!(read1, value2);
        assert_eq!(read2, value2);
        assert_eq!(read3, value2);

        // Test 4: Node failure handling
        let key3 = "key3".to_string();
        let value3 = b"value3".to_vec();

        // Write initial value
        node1.write(key3.clone(), value3.clone()).await?;

        // Simulate node2 failure
        node2.set_state(NodeState::Inactive);

        // Should still be able to read/write with remaining nodes
        let read1 = node1.read(key3.clone()).await?;
        let read3 = node3.read(key3.clone()).await?;
        assert_eq!(read1, value3);
        assert_eq!(read3, value3);

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    };

    // Run test with timeout
    tokio::time::timeout(std::time::Duration::from_secs(5), test)
        .await
        .map_err(|_| Box::<dyn std::error::Error + Send + Sync>::from("Test timed out"))??;

    Ok(())
}
