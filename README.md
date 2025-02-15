# Hermes Replication Protocol

Hermes is a high-performance, strongly-consistent replication protocol designed for distributed in-memory systems. It provides sequential consistency while optimizing for low latency and network efficiency.

## Overview

Hermes uses a two-phase protocol to ensure all nodes maintain consistent data:

1. **Invalidation Phase**
   - Writing node sends invalidation messages for specific keys
   - All nodes must acknowledge invalidations
   - Prevents stale reads during updates

2. **Data Transmission Phase**
   - After receiving all ACKs, writer sends new data
   - Data transmitted only to nodes that acknowledged invalidation
   - Ensures all nodes have consistent view of data

## Key Features

- **Strong Consistency**: Sequential consistency guarantees with no stale reads
- **Network Efficient**: Minimizes data transmission through targeted invalidations
- **High Performance**: Optimized for in-memory operations
- **Fault Tolerant**: Handles node failures and network partitions

## Use Cases

- Distributed caching systems
- In-memory databases
- High-performance distributed storage
- Systems requiring strong consistency guarantees

## Benefits

- Lower network bandwidth usage compared to traditional protocols
- Reduced latency through optimized two-phase approach
- Strong consistency without sacrificing performance
- Scalable to large number of nodes

## Implementation Requirements

- Reliable network communication between nodes
- Membership management system
- Failure detection mechanism
- State management for tracking invalidations

## References

- [Original Hermes Paper](https://www.usenix.org/system/files/nsdi20-paper-katsarakis.pdf)
- [ACM SIGOPS Operating Systems Review](https://dl.acm.org/doi/10.1145/3373376.3378468)
