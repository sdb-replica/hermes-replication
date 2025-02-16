//! Hermes Replication Library
//!
//! This library implements the Hermes replication protocol, which provides:
//! - Linearizable reads and writes
//! - High availability through replication
//! - Low latency local reads
//! - Strong consistency guarantees
//!
//! # Architecture
//!
//! The system consists of:
//! - ClusterNode: Main node implementation that handles replication
//! - NetworkServer/Client: Network communication layer
//! - Protocol: Core Hermes protocol implementation
//!
//! # Basic Usage
//!
//! ```rust
//! use hermes_replication::ClusterNode;
//! use std::net::SocketAddr;
//!
//! // Create a node
//! let node = ClusterNode::new(
//!     "127.0.0.1:8080".parse().unwrap()
//! );
//!
//! // Write and read data
//! async {
//!     node.write("key".to_string(), b"value".to_vec()).await?;
//!     let value = node.read("key".to_string()).await?;
//!     assert_eq!(value, b"value");
//!     Ok::<(), Box<dyn std::error::Error>>(())
//! };
//! ```

pub mod network;
pub mod protocol;
pub mod types;

// Re-export only what's needed by external users
pub use protocol::ClusterNode;
pub use types::{ClusterMessage, HermesError, NodeInfo, NodeState};
