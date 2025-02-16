//! Core types used throughout the Hermes replication system.

use rkyv;
use serde;
use std::fmt;
use std::net::SocketAddr;
use uuid::Uuid;

#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
#[archive_attr(derive(Debug, PartialEq))]
pub enum NodeState {
    Active,
    Inactive,
    Suspected,
}

#[derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, Debug, Clone)]
#[archive_attr(derive(Debug, PartialEq))]
pub enum ClusterMessage {
    // Membership messages
    JoinRequest(NodeInfo),
    JoinResponse(Vec<NodeInfo>),
    HeartBeat,
    HeartBeatAck,

    // Hermes protocol messages
    Invalidation {
        key: String,
        version: u64,
    },
    InvalidationAck {
        key: String,
        version: u64,
    },
    DataUpdate {
        key: String,
        value: Vec<u8>,
        version: u64,
    },
    Validation {
        key: String,
        value: Vec<u8>,
        version: u64,
    },
    ValidationAck {
        key: String,
        version: u64,
    },
    ReadRequest {
        key: String,
    },
    ReadResponse {
        key: String,
        value: Vec<u8>,
        version: u64,
    },
}

#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
#[archive_attr(derive(Debug, PartialEq))]
pub struct NodeInfo {
    pub id: NodeId,
    pub address: SocketAddr,
    pub state: NodeState,
}

/// Unique identifier for a node in the cluster
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
#[archive_attr(derive(Debug, PartialEq))]
pub struct NodeId(pub Uuid);

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeId {
    /// Creates a new random NodeId
    pub fn new() -> Self {
        NodeId(Uuid::new_v4())
    }
}

/// Errors that can occur during Hermes operations
#[derive(Debug)]
pub enum HermesError {
    /// Node is not responsible for the requested key
    NotResponsible,

    /// No active replicas available for operation
    NoActiveReplicas,

    /// Network communication error
    NetworkError(String),

    /// Failed to achieve required quorum
    QuorumFailed(String),

    /// Requested value not found
    ValueNotFound(String),
}

impl fmt::Display for HermesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HermesError::NotResponsible => write!(f, "Node not responsible for key"),
            HermesError::NoActiveReplicas => write!(f, "No active replicas available"),
            HermesError::NetworkError(e) => write!(f, "Network error: {}", e),
            HermesError::QuorumFailed(e) => write!(f, "Failed to achieve quorum: {}", e),
            HermesError::ValueNotFound(key) => write!(f, "Value not found for key: {}", key),
        }
    }
}

impl std::error::Error for HermesError {}
