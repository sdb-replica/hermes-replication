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
    node.write("key".to_string(), b"value".to_vec()).await?;
    let value = node.read("key".to_string()).await?;
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

### Node States
- Active: Node can participate in operations
- Inactive: Node cannot participate in operations
- Suspected: Node is potentially unhealthy (not fully implemented)

### Error Handling
- NotResponsible: Node is not coordinator for key
- NoActiveReplicas: No active nodes available
- NetworkError: Communication failure
- QuorumFailed: Failed to reach required replicas
- ValueNotFound: Key does not exist

## Testing

### Cluster Tests
- Multi-node cluster setup
- Write/read operations
- Node state changes
- Membership changes

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
⏳ Persistence (TODO)
⏳ Node recovery (TODO)
⏳ Consistent hashing for replica selection (TODO)
⏳ Full invalidation phase (TODO)

## References
- [Original Hermes Paper](https://arxiv.org/pdf/2001.09804)
