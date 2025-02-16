# Hermes Replication Protocol

Hermes is a high-performance, strongly-consistent replication protocol designed for distributed in-memory systems. It provides sequential consistency while optimizing for low latency and network efficiency.

## Overview

Hermes uses a simplified protocol for testing that focuses on direct validation of writes:

1. **Write Protocol**
   - Coordinator determines replica set for key
   - Sends validation messages to all replicas
   - Waits for acknowledgments from all replicas
   - Updates local data after successful validation

2. **Read Protocol**
   - Node checks if it's a replica for the key
   - Returns local value if valid
   - If local copy invalid/missing, fetches from another replica
   - Updates local copy with fetched value

## Detailed Operation Flows

### Write Flow
1. Client initiates write to any node
2. Node checks if it's a replica for the key
3. If yes:
   - Generates new version number
   - Sends validation messages to all other replicas
   - Waits for ValidationAck from each replica
   - Updates local data after all acks received
4. If no:
   - Returns error "not responsible for key"

### Read Flow
1. Client initiates read to any node
2. Node checks if it's a replica for the key
3. If yes:
   - Returns local value if present and valid
   - If value missing/invalid:
     - Finds another replica
     - Requests value from that replica
     - Updates local copy
     - Returns value to client
4. If no:
   - Returns error "not responsible for key"

## Implementation Details

### Node States
- Active: Node is functioning normally
- Inactive: Node is temporarily unavailable

### Message Types
- Validation: Contains new key-value pair and version
- ValidationAck: Confirms successful validation
- ReadRequest: Requests value for key
- ReadResponse: Returns value and version
- JoinRequest/Response: Handles cluster membership

## Testing

### Cluster Tests (tests/cluster_test.rs)
- Multi-node cluster setup
- Write/read operations
- Node state changes
- Membership changes

### Failure Tests (tests/failure_test.rs)
- Write failures (NotResponsible, NoActiveReplicas)
- Read failures (ValueNotFound)
- Network failures
- Quorum failures

## Implementation Status

✅ Basic replication protocol
✅ Decentralized write coordination
✅ Local reads from valid copies
✅ Error handling
✅ Network communication
✅ Test coverage
⏳ Persistence (TODO)
⏳ Node recovery (TODO)
⏳ Consistent hashing for replica selection (TODO)
⏳ Full invalidation phase (TODO)

## Future Improvements
- Implement full invalidation phase
- Add persistent storage
- Improve replica selection with consistent hashing
- Add failure recovery mechanisms

## References
- [Original Hermes Paper](https://www.usenix.org/system/files/nsdi20-paper-katsarakis.pdf)
