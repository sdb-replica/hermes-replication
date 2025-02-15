use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use uuid::Uuid;
use crate::types::{NodeRole, NodeState, NodeInfo, ClusterMessage};

/// Represents a node in the Hermes cluster
#[derive(Debug)]
pub struct ClusterNode {
    /// Node's own information
    pub info: NodeInfo,
    
    /// Known cluster members
    pub members: Arc<RwLock<HashMap<Uuid, NodeInfo>>>,
    
    /// Local data store
    data: Arc<RwLock<HashMap<String, (Vec<u8>, u64)>>>, // (value, version)
    
    /// Pending invalidations
    pending_invalidations: Arc<RwLock<HashMap<String, Vec<Uuid>>>>, // key -> list of nodes that need to ack
}

impl ClusterNode {
    pub fn new(address: SocketAddr, role: NodeRole) -> Self {
        let info = NodeInfo {
            id: Uuid::new_v4(),
            address,
            role,
            state: NodeState::Active,
        };

        ClusterNode {
            info: info.clone(),
            members: Arc::new(RwLock::new(HashMap::from([(info.id, info)]))),
            data: Arc::new(RwLock::new(HashMap::new())),
            pending_invalidations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn handle_message(&self, _from: Uuid, message: ClusterMessage) -> Option<ClusterMessage> {
        match message {
            ClusterMessage::JoinRequest(node_info) => {
                self.members.write().unwrap().insert(node_info.id, node_info);
                Some(ClusterMessage::JoinResponse(true))
            }
            ClusterMessage::Invalidation { key, version } => {
                Some(ClusterMessage::InvalidationAck { key, version })
            }
            ClusterMessage::HeartBeat => {
                Some(ClusterMessage::HeartBeatAck)
            }
            _ => None,
        }
    }

    pub async fn write(&self, key: String, _value: Vec<u8>) -> Result<(), String> {
        let _version = self.get_next_version(&key);
        
        let members_guard = self.members.read().unwrap();
        let nodes_to_invalidate: Vec<Uuid> = members_guard.keys()
            .filter(|&&id| id != self.info.id)
            .cloned()
            .collect();
        drop(members_guard);

        self.pending_invalidations.write().unwrap()
            .insert(key.clone(), nodes_to_invalidate);

        Ok(())
    }

    fn get_next_version(&self, key: &str) -> u64 {
        self.data.read().unwrap().get(key).map(|(_, v)| v + 1).unwrap_or(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_cluster_node_creation() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let node = ClusterNode::new(addr, NodeRole::Leader);
        
        assert_eq!(node.members.read().unwrap().len(), 1);
        assert_eq!(node.info.role, NodeRole::Leader);
        assert_eq!(node.info.state, NodeState::Active);
    }
} 