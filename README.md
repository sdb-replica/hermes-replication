# Hermes Replication Protocol

A Rust implementation of a distributed replication protocol providing:
- Strong consistency (linearizability)
- High availability through replication
- Low latency local reads
- Decentralized peer-to-peer architecture

## Overview

```rust
use hermes_replication::ClusterNode;
use std::net::SocketAddr;

// Create a node
let node = ClusterNode::new(
    "127.0.0.1:8080".parse().unwrap()
);

// Write and read data
async {
    node.write(b"key".to_vec(), b"value".to_vec()).await?;
    let value = node.read(b"key".to_vec()).await?;
    assert_eq!(value, b"value");
};
```

## Protocol Details

### Write Protocol
1. Coordinator Selection:
   - Each key maps to a specific coordinator node
   - Mapping uses consistent hashing of the key
   - Any node can be coordinator for different keys

2. Write operation flow:
   - Node checks if it's coordinator for the key
   - Coordinator sends validation to all active replicas
   - Updates local copy after successful validation
   - Write completes when all replicas acknowledge

### Read Protocol
1. Any active node can serve reads if:
   - Node has valid local copy
   - Node is in active replica set
2. If local copy invalid:
   - Fetch from other replicas
   - Update local copy
   - Serve read

### Version Management
- Globally unique versions using node ID and counter
- High 16 bits: Node identifier
- Low 48 bits: Monotonic counter
- Ensures consistent ordering of concurrent writes

### Error Handling
- NotResponsible: Node is not coordinator for key
- NoActiveReplicas: No active nodes available
- NetworkError: Communication failure
- QuorumFailed: Failed to reach required replicas
- ValueNotFound: Key does not exist

### Node States
- Active: Node can participate in operations
- Inactive: Node cannot participate in operations
- Suspected: Node is potentially unhealthy

## Testing

### Test Summary

| # | Test Case | Description | Expected Result |
|---|-----------|-------------|-----------------|
| **Cluster Tests** ||||
| 1 | test_three_node_cluster | Initialize a 3-node cluster and verify basic network connectivity and membership protocols | • Successful node joins<br>• Heartbeat responses<br>• Complete membership list |
| 2 | test_write_and_read | Perform coordinated write operation and verify data consistency across all nodes | • Write completes successfully<br>• All nodes have identical data<br>• Correct version numbers |
| 3 | test_concurrent_writes | Execute multiple simultaneous writes to the same key to test conflict resolution | • All nodes converge to same value<br>• Consistent version ordering<br>• No data corruption |
| **Hermes Protocol Tests** ||||
| 4.1 | Coordinator Selection | Verify that only the designated coordinator can write to a key | • Write from non-coordinator fails with NotResponsible<br>• Write from coordinator succeeds |
| 4.2 | Write Protocol & Versioning | Test write replication and version synchronization | • All nodes receive the write<br>• All nodes have identical version numbers<br>• Version numbers match across replicas |
| 4.3 | Local vs Forwarded Reads | Compare reads from coordinator and non-coordinator nodes | • Both reads return same value<br>• Coordinator reads succeed<br>• Non-coordinator reads succeed |
| 4.4 | Version Increment | Verify version numbers increment properly on updates | • New version > old version<br>• All nodes update to new version<br>• Data consistency maintained |
| 4.5 | Multiple Key Operations | Test operations on multiple keys with different coordinators | • Each key maintains correct value<br>• Different coordinators can operate independently<br>• No cross-key interference |
| **Failure Tests** ||||
| 5 | test_write_failures | Test write operation failure modes:<br>• Write to wrong coordinator<br>• Write to inactive cluster | • NotResponsible error when wrong coordinator<br>• NoActiveReplicas error when cluster inactive |
| 6 | test_read_failures | Test read operation failure modes:<br>• Read non-existent data<br>• Read from inactive node | • ValueNotFound error for missing data<br>• NoActiveReplicas error for inactive node |
| 7 | test_network_failures | Simulate network partition by disconnecting nodes during operation | • QuorumFailed error when unable to reach sufficient replicas<br>• Consistent state after recovery |
| 8 | test_quorum_failures | Test behavior when insufficient nodes are available for consensus | • QuorumFailed error when consensus impossible<br>• No partial updates |

### Test Coverage
- Multi-node cluster setup
- Write/read operations
- Node state changes
- Membership changes
- Concurrent write handling

### Failure Tests
- Write coordination failures
- Read failures
- Network failures
- Quorum failures

## Implementation Status

✅ Basic replication protocol

✅ Key-based coordinator selection

✅ Local reads from valid copies

✅ Error handling

✅ Network communication

✅ Test coverage

✅ Concurrent write handling

✅ Version management

⏳ Persistence (TODO)

⏳ Node recovery (TODO)

⏳ Consistent hashing for replica selection (TODO)

⏳ Full invalidation phase (TODO)

## References
- [Original Hermes Paper](https://arxiv.org/pdf/2001.09804)
