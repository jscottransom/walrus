use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Node ID for this instance
    pub node_id: String,
    /// Bind address for this node
    pub bind_addr: SocketAddr,
    /// List of all cluster nodes (including this one)
    pub nodes: HashMap<String, SocketAddr>,
    /// Election timeout in milliseconds
    pub election_timeout_ms: u64,
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
    /// Replication timeout in milliseconds
    pub replication_timeout_ms: u64,
    /// Data directory for WAL storage
    pub data_dir: String,
    /// Maximum segment size in bytes
    pub max_segment_bytes: u64,
    /// Maximum index size in bytes
    pub max_index_bytes: u64,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            bind_addr: "127.0.0.1:8080".parse().unwrap(),
            nodes: HashMap::new(),
            election_timeout_ms: 1000,
            heartbeat_interval_ms: 100,
            replication_timeout_ms: 5000,
            data_dir: "/tmp/walrus".to_string(),
            max_segment_bytes: 1024 * 1024, // 1MB
            max_index_bytes: 1024 * 1024,   // 1MB
        }
    }
}

impl ClusterConfig {
    pub fn new(node_id: String, bind_addr: SocketAddr) -> Self {
        let mut config = Self::default();
        config.node_id = node_id;
        config.bind_addr = bind_addr;
        config
    }

    pub fn add_node(&mut self, node_id: String, addr: SocketAddr) {
        self.nodes.insert(node_id, addr);
    }

    pub fn election_timeout(&self) -> Duration {
        Duration::from_millis(self.election_timeout_ms)
    }

    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_millis(self.heartbeat_interval_ms)
    }

    pub fn replication_timeout(&self) -> Duration {
        Duration::from_millis(self.replication_timeout_ms)
    }
}
