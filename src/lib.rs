pub mod types;
pub mod protocol;
pub mod network;

// Re-export only what's needed by external users
pub use protocol::ClusterNode;
pub use types::{ClusterMessage, NodeInfo, NodeRole};