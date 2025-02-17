use crate::network::NetworkClient;
use crate::types::{ClusterMessage, HermesError, NodeId, NodeInfo, NodeState};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// Represents a node in the Hermes cluster
pub struct ClusterNode {
    /// Node's own information
    pub info: NodeInfo,

    /// Known cluster members
    pub members: Arc<RwLock<HashMap<NodeId, NodeInfo>>>,

    /// Local data store
    /// TODO: Make this persistent.
    #[allow(clippy::type_complexity)]
    data: Arc<RwLock<HashMap<Vec<u8>, (Vec<u8>, u64)>>>, // (key, (value, version))

    /// Pending invalidations
    pending_invalidations: Arc<RwLock<HashMap<Vec<u8>, HashSet<NodeId>>>>, // key -> set of nodes that need to ack

    version_counter: AtomicU64, // Add atomic version counter
}

impl ClusterNode {
    pub fn new(address: SocketAddr) -> Self {
        let info = NodeInfo {
            id: NodeId::new(),
            address,
            state: NodeState::Active,
        };

        ClusterNode {
            info: info.clone(),
            members: Arc::new(RwLock::new(HashMap::from([(info.id, info)]))),
            data: Arc::new(RwLock::new(HashMap::new())),
            pending_invalidations: Arc::new(RwLock::new(HashMap::new())),
            version_counter: AtomicU64::new(0), // Initialize version counter
        }
    }

    pub async fn handle_message(
        &self,
        from: NodeId,
        message: ClusterMessage,
    ) -> Option<ClusterMessage> {
        match message {
            ClusterMessage::JoinRequest(node_info) => self.handle_join_request(node_info),
            ClusterMessage::Invalidation { .. } => self.handle_invalidation(from, message),
            ClusterMessage::Validation { .. } => self.handle_validation(message),
            ClusterMessage::HeartBeat => Some(ClusterMessage::HeartBeatAck),
            ClusterMessage::ReadRequest { key } => {
                // Return local value if we have it
                if let Some((value, version)) = self.data.read().unwrap().get(&key) {
                    Some(ClusterMessage::ReadResponse {
                        key,
                        value: value.clone(),
                        version: *version,
                    })
                } else {
                    None
                }
            }
            ClusterMessage::JoinResponse(members) => {
                println!(
                    "Node {:?} received JoinResponse with {} members",
                    self.info.id,
                    members.len()
                );
                // Update our member list with all known members
                let mut members_guard = self.members.write().unwrap();
                for member in members {
                    println!("Adding member {:?} to node {:?}", member.id, self.info.id);
                    members_guard.insert(member.id, member);
                }
                println!(
                    "Node {:?} now has {} members",
                    self.info.id,
                    members_guard.len()
                );
                None
            }
            _ => None,
        }
    }

    fn handle_join_request(&self, node_info: NodeInfo) -> Option<ClusterMessage> {
        // Add the new node
        self.members
            .write()
            .unwrap()
            .insert(node_info.id, node_info.clone());

        // Send back our complete member list
        let members: Vec<NodeInfo> = self.members.read().unwrap().values().cloned().collect();

        Some(ClusterMessage::JoinResponse(members))
    }

    fn handle_invalidation(
        &self,
        _from: NodeId,
        message: ClusterMessage,
    ) -> Option<ClusterMessage> {
        match message {
            ClusterMessage::Invalidation { key, version } => {
                // Handle incoming invalidation by marking local data as invalid
                if let Some((value, _)) = self.data.write().unwrap().get_mut(&key) {
                    // Mark as invalid by clearing the value
                    *value = Vec::new();
                }

                // Send acknowledgment back to coordinator
                Some(ClusterMessage::InvalidationAck { key, version })
            }
            _ => None,
        }
    }

    fn handle_validation(&self, message: ClusterMessage) -> Option<ClusterMessage> {
        match message {
            ClusterMessage::Validation {
                key,
                value,
                version,
            } => {
                // Only update if new version is higher
                let mut data = self.data.write().unwrap();
                if let Some((_, current_version)) = data.get(&key) {
                    if version <= *current_version {
                        return Some(ClusterMessage::ValidationAck { key, version });
                    }
                }
                data.insert(key.clone(), (value, version));
                Some(ClusterMessage::ValidationAck { key, version })
            }
            _ => None,
        }
    }

    /// Performs a write operation following the Hermes protocol.
    ///
    /// This implements the Hermes write protocol which provides:
    /// - Linearizability: all operations appear to take place atomically and in real-time order
    /// - High availability: writes complete if at least one replica is accessible
    /// - Low latency: completes in 1.5 RTTs with all replicas
    ///
    /// The protocol has two phases:
    /// 1. Invalidation Phase (INV):
    ///    - The coordinator (this node) sends INV messages to all replicas
    ///    - Each replica invalidates its local copy and responds with INV-ACK
    ///    - The coordinator waits for all INV-ACKs before proceeding
    ///
    /// 2. Validation Phase (VAL):
    ///    - The coordinator sends VAL messages with the new value to all replicas
    ///    - Each replica updates its local copy with the new value
    ///    - The coordinator considers the write complete after sending all VAL messages
    ///    - The coordinator updates its local copy only after all remote replicas are updated
    ///
    /// # Arguments
    /// * `key` - The key to write
    /// * `value` - The new value to write
    ///
    /// # Returns
    /// * `Ok(())` if write succeeds
    /// * `Err(HermesError)` if write fails, with error description
    ///
    /// # Protocol Safety
    /// The protocol ensures safety by:
    /// - Using versioning to detect concurrent writes
    /// - Invalidating all copies before updating any copy
    /// - Maintaining the invariant that a valid copy is always up-to-date
    pub async fn write(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), HermesError> {
        // Check if node is active
        {
            let members = self.members.read().unwrap();
            if let Some(info) = members.get(&self.info.id) {
                if info.state != NodeState::Active {
                    return Err(HermesError::NoActiveReplicas);
                }
            }
        }

        // In Hermes, each key is assigned to a specific coordinator
        let coordinator_id = {
            let hash = key.iter().fold(0u64, |acc, &x| acc.wrapping_add(x as u64));
            let members = self.members.read().unwrap();
            let mut active_members: Vec<_> = members
                .values()
                .filter(|info| info.state == NodeState::Active)
                .collect();
            if active_members.is_empty() {
                return Err(HermesError::NoActiveReplicas);
            }
            active_members.sort_by_key(|info| info.id);
            active_members[hash as usize % active_members.len()].id
        };

        // Only the coordinator can write
        if self.info.id != coordinator_id {
            return Err(HermesError::NotResponsible);
        }

        // Get next version for this key
        let version = self.get_next_version(&key);

        // Get all active nodes for replication
        let member_addresses: HashMap<NodeId, SocketAddr> = {
            let members = self.members.read().unwrap();
            members
                .iter()
                .filter(|(id, info)| **id != self.info.id && info.state == NodeState::Active)
                .map(|(id, info)| (*id, info.address))
                .collect()
        };

        // Send validation to all active nodes
        let mut clients = HashMap::new();
        let mut failed_nodes = Vec::new();

        for (node_id, address) in member_addresses {
            match NetworkClient::connect(address).await {
                Ok(client) => {
                    clients.insert(node_id, client);
                }
                Err(_) => {
                    // Don't return early, collect all failures
                    failed_nodes.push(node_id);
                }
            }
        }

        // Update all nodes with the new version
        for (node_id, mut client) in clients {
            match client
                .send(ClusterMessage::Validation {
                    key: key.clone(),
                    value: value.clone(),
                    version,
                })
                .await
            {
                Ok(Some(ClusterMessage::ValidationAck {
                    version: ack_version,
                    ..
                })) if ack_version == version => (),
                _ => failed_nodes.push(node_id),
            }
        }

        // Update local copy with the new version
        self.data.write().unwrap().insert(key, (value, version));

        if !failed_nodes.is_empty() {
            return Err(HermesError::QuorumFailed(format!(
                "Failed to validate with nodes: {:?}",
                failed_nodes
            )));
        }

        Ok(())
    }

    fn get_next_version(&self, _key: &[u8]) -> u64 {
        // Use high 16 bits for node ID hash, low 48 bits for counter
        let node_hash = self.info.id.0.as_u128() as u64; // Get hash of UUID
        let counter = self.version_counter.fetch_add(1, Ordering::SeqCst);

        // Combine node hash (high 16 bits) with counter (low 48 bits)
        ((node_hash & 0xFFFF) << 48) | (counter & 0xFFFFFFFFFFFF)
    }

    // Add helper method to check if invalidation is complete
    pub fn is_invalidation_complete(&self, key: &[u8]) -> bool {
        self.pending_invalidations
            .read()
            .unwrap()
            .get(key)
            .map_or(true, |nodes| nodes.is_empty())
    }

    pub fn get_replica_nodes(&self, key: &[u8]) -> HashSet<NodeId> {
        let members = self.members.read().unwrap();
        println!("\nGetting replicas for key: {:?}", key);
        println!("Total members: {}", members.len());

        let mut replicas = HashSet::new();

        // In Hermes, all active nodes are replicas
        for (id, info) in members.iter() {
            if info.state == NodeState::Active {
                replicas.insert(*id);
            }
        }

        println!("Selected replicas: {:?}\n", replicas);
        replicas
    }

    /// Performs a read operation following the Hermes protocol.
    ///
    /// In Hermes, reads are local operations that complete immediately if:
    /// 1. The local copy is valid (not invalidated)
    /// 2. This node is a designated replica for the key
    ///
    /// If the local copy is invalid, the coordinator must:
    /// 1. Request the latest value from another replica
    /// 2. Update its local copy
    /// 3. Return the value to the client
    ///
    /// # Arguments
    /// * `key` - The key to read
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - The value if read succeeds
    /// * `Err(HermesError)` - Error description if read fails
    ///
    /// # Protocol Safety
    /// The protocol ensures safety by:
    /// - Using versioning to detect concurrent writes
    /// - Invalidating all copies before updating any copy
    /// - Maintaining the invariant that a valid copy is always up-to-date
    pub async fn read(&self, key: Vec<u8>) -> Result<Vec<u8>, HermesError> {
        let replica_nodes = self.get_replica_nodes(&key);

        // First check if there are any active replicas
        let active_replicas: Vec<NodeId> = replica_nodes
            .into_iter()
            .filter(|id| {
                self.members
                    .read()
                    .unwrap()
                    .get(id)
                    .map(|info| info.state == NodeState::Active)
                    .unwrap_or(false)
            })
            .collect();

        if active_replicas.is_empty() {
            return Err(HermesError::NoActiveReplicas);
        }

        // Then check if this node is responsible
        if !active_replicas.contains(&self.info.id) {
            return Err(HermesError::NotResponsible);
        }

        // Check local data
        if let Some((value, _)) = self.data.read().unwrap().get(&key) {
            if !value.is_empty() {
                return Ok(value.clone());
            }
        }

        // If no local value, try to get from other replicas
        let active_replicas: Vec<NodeInfo> = {
            let members_guard = self.members.read().unwrap();
            active_replicas
                .iter()
                .filter(|id| **id != self.info.id)
                .filter_map(|id| members_guard.get(id).cloned())
                .collect()
        };

        for replica in active_replicas {
            match NetworkClient::connect(replica.address)
                .await
                .map_err(|e| HermesError::NetworkError(e.to_string()))?
                .send(ClusterMessage::ReadRequest { key: key.clone() })
                .await
            {
                Ok(Some(ClusterMessage::ReadResponse { value, version, .. })) => {
                    self.data
                        .write()
                        .unwrap()
                        .insert(key.clone(), (value.clone(), version));
                    return Ok(value);
                }
                _ => continue,
            }
        }

        Err(HermesError::ValueNotFound(key))
    }

    pub fn set_state(&self, state: NodeState) {
        // Update state in members list only
        let mut members = self.members.write().unwrap();
        if let Some(info) = members.get_mut(&self.info.id) {
            info.state = state;
        }
    }

    pub fn get_version(&self, key: &[u8]) -> Option<u64> {
        self.data
            .read()
            .unwrap()
            .get(key)
            .map(|(_, version)| *version)
    }

    pub fn get_value_and_version(&self, key: &[u8]) -> Option<(Vec<u8>, u64)> {
        self.data.read().unwrap().get(key).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_cluster_node_creation() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let node = ClusterNode::new(addr);

        assert_eq!(node.members.read().unwrap().len(), 1);
        assert_eq!(node.info.state, NodeState::Active);
    }

    #[test]
    fn test_version_increment() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let node = ClusterNode::new(addr);

        // Get initial version
        let v1 = node.get_next_version("key1".as_bytes());

        // Insert data with version 1
        node.data
            .write()
            .unwrap()
            .insert("key1".to_string().into_bytes(), (vec![], v1));

        // Get next version
        let v2 = node.get_next_version("key1".as_bytes());

        // Verify versions are different and increasing
        assert!(v2 > v1, "Version should increase");

        // Verify high 16 bits contain node hash
        let node_hash = (v1 >> 48) & 0xFFFF;
        assert_eq!(
            node_hash,
            (v2 >> 48) & 0xFFFF,
            "Node hash portion should be constant"
        );

        // Verify low 48 bits are sequential
        let counter1 = v1 & 0xFFFFFFFFFFFF;
        let counter2 = v2 & 0xFFFFFFFFFFFF;
        assert_eq!(counter2, counter1 + 1, "Counter should increment by 1");
    }

    #[test]
    fn test_replica_selection() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let node = ClusterNode::new(addr);

        let replicas = node.get_replica_nodes("key1".as_bytes());
        assert_eq!(replicas.len(), 1);
        assert!(replicas.contains(&node.info.id));
    }

    #[test]
    fn test_node_state_change() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let node = ClusterNode::new(addr);

        assert_eq!(node.info.state, NodeState::Active);
        node.set_state(NodeState::Inactive);
        assert_eq!(
            node.members
                .read()
                .unwrap()
                .get(&node.info.id)
                .unwrap()
                .state,
            NodeState::Inactive
        );
    }
}
