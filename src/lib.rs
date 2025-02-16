pub mod network;
pub mod protocol;
pub mod types;

// Re-export only what's needed by external users
pub use protocol::ClusterNode;
pub use types::{ClusterMessage, NodeInfo, NodeRole};
