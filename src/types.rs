use rkyv;
use serde;
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
pub enum NodeRole {
    Leader,
    Follower,
}

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
    pub role: NodeRole,
    pub state: NodeState,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
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
    pub fn new() -> Self {
        NodeId(Uuid::new_v4())
    }
}
