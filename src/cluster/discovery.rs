use crate::cluster::state::{ClusterStateManager, NodeInfo, NodeRole};
use crate::cluster::config::ClusterConfig;
use anyhow::Result;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct NodeRegistration {
    pub node_id: String,
    pub addr: SocketAddr,
    pub role: NodeRole,
}

#[derive(Clone)]
pub struct DiscoveryManager {
    config: ClusterConfig,
    state_manager: Arc<ClusterStateManager>,
    known_nodes: Arc<RwLock<HashMap<String, SocketAddr>>>,
}

impl DiscoveryManager {
    pub fn new(config: ClusterConfig, state_manager: Arc<ClusterStateManager>) -> Self {
        Self {
            config,
            state_manager,
            known_nodes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_discovery(&self) -> Result<()> {
        info!("Starting node discovery for node {}", self.config.node_id);
        
        // Register self
        self.register_node(&self.config.node_id, self.config.bind_addr).await?;
        
        // Start discovery loop
        self.discovery_loop().await?;
        
        Ok(())
    }

    async fn discovery_loop(&self) -> Result<()> {
        loop {
            // Check for new nodes in the cluster
            self.discover_nodes().await?;
            
            // Check for dead nodes
            self.check_node_health().await?;
            
            // Wait before next discovery round
            sleep(Duration::from_secs(5)).await;
        }
    }

    async fn discover_nodes(&self) -> Result<()> {
        let known_nodes = self.known_nodes.read().await;
        
        for (node_id, addr) in known_nodes.iter() {
            if node_id == &self.config.node_id {
                continue;
            }
            
            // Try to ping the node
            if let Ok(true) = self.ping_node(node_id, addr).await {
                // Node is alive, ensure it's in our state
                self.ensure_node_registered(node_id, addr).await?;
            } else {
                // Node might be dead
                warn!("Node {} at {} appears to be unreachable", node_id, addr);
            }
        }
        
        Ok(())
    }

    async fn ping_node(&self, node_id: &str, addr: &SocketAddr) -> Result<bool> {
        // This would typically use gRPC to ping the node
        // For now, we'll simulate the response
        let state = self.state_manager.get_state();
        
        if let Some(node) = state.nodes.get(node_id) {
            // Simulate successful ping if node is marked as alive
            Ok(node.is_alive)
        } else {
            // Node not in state, assume it's reachable
            Ok(true)
        }
    }

    async fn ensure_node_registered(&self, node_id: &str, addr: &SocketAddr) -> Result<()> {
        let state = self.state_manager.get_state();
        
        if !state.nodes.contains_key(node_id) {
            let node_info = NodeInfo::new(node_id.to_string(), *addr);
            self.state_manager.add_node(node_info)?;
            info!("Registered new node {} at {}", node_id, addr);
        }
        
        Ok(())
    }

    async fn check_node_health(&self) -> Result<()> {
        let state = self.state_manager.get_state();
        let now = std::time::Instant::now();
        
        for (node_id, node_info) in &state.nodes {
            if node_id == &self.config.node_id {
                continue;
            }
            
            if let Some(last_heartbeat) = node_info.last_heartbeat {
                let elapsed = now.duration_since(last_heartbeat);
                
                // Mark node as dead if no heartbeat for 3x election timeout
                if elapsed > self.config.election_timeout() * 3 {
                    self.state_manager.mark_node_dead(node_id)?;
                    warn!("Marked node {} as dead (no heartbeat for {:?})", node_id, elapsed);
                }
            }
        }
        
        Ok(())
    }

    pub async fn register_node(&self, node_id: &str, addr: SocketAddr) -> Result<()> {
        // Add to known nodes
        {
            let mut known_nodes = self.known_nodes.write().await;
            known_nodes.insert(node_id.to_string(), addr);
        }
        
        // Add to cluster state
        let node_info = NodeInfo::new(node_id.to_string(), addr);
        self.state_manager.add_node(node_info)?;
        
        info!("Registered node {} at {}", node_id, addr);
        Ok(())
    }

    pub async fn unregister_node(&self, node_id: &str) -> Result<()> {
        // Remove from known nodes
        {
            let mut known_nodes = self.known_nodes.write().await;
            known_nodes.remove(node_id);
        }
        
        // Remove from cluster state
        self.state_manager.remove_node(node_id)?;
        
        info!("Unregistered node {}", node_id);
        Ok(())
    }

    pub async fn get_cluster_nodes(&self) -> Vec<NodeInfo> {
        let state = self.state_manager.get_state();
        state.nodes.values().cloned().collect()
    }

    pub async fn get_alive_nodes(&self) -> Vec<String> {
        self.state_manager.get_alive_nodes()
    }

    pub async fn is_node_alive(&self, node_id: &str) -> bool {
        let state = self.state_manager.get_state();
        state.nodes.get(node_id).map(|n| n.is_alive).unwrap_or(false)
    }

    pub async fn update_node_heartbeat(&self, node_id: &str) -> Result<()> {
        self.state_manager.update_heartbeat(node_id)
    }

    pub async fn get_node_info(&self, node_id: &str) -> Option<NodeInfo> {
        let state = self.state_manager.get_state();
        state.nodes.get(node_id).cloned()
    }
}
