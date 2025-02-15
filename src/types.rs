use serde;
use std::net::SocketAddr;
use uuid::Uuid;
use rkyv;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[archive_attr(derive(Debug, PartialEq))]
pub enum NodeRole {
    Leader,
    Follower,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
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
    JoinResponse(bool),
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
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
#[archive_attr(derive(Debug, PartialEq))]
pub struct NodeInfo {
    pub id: Uuid,
    pub address: SocketAddr,
    pub role: NodeRole,
    pub state: NodeState,
} 