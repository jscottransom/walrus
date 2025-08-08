use crate::client::grpc::WalClient;
use crate::cluster::config::ClusterConfig;
use anyhow::Result;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

pub struct WalClientService {
    clients: Arc<RwLock<HashMap<String, WalClient>>>,
    config: ClusterConfig,
    current_leader: Arc<RwLock<Option<String>>>,
}

impl WalClientService {
    pub fn new(config: ClusterConfig) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            config,
            current_leader: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn connect_to_node(&mut self, node_id: &str, addr: SocketAddr) -> Result<()> {
        let client = WalClient::new(addr).await?;
        
        let mut clients = self.clients.write().await;
        clients.insert(node_id.to_string(), client);
        
        info!("Connected to node {} at {}", node_id, addr);
        Ok(())
    }

    pub async fn write(&mut self, data: Vec<u8>) -> Result<u64> {
        // Try to find the leader
        let leader = self.find_leader().await?;
        
        let mut clients = self.clients.write().await;
        if let Some(client) = clients.get_mut(&leader) {
            match client.write(data, 0).await {
                Ok(offset) => {
                    info!("Successfully wrote data to leader {} at offset {}", leader, offset);
                    Ok(offset)
                }
                Err(e) => {
                    error!("Failed to write to leader {}: {}", leader, e);
                    // Try to find a new leader
                    self.current_leader.write().await.take();
                    Err(e)
                }
            }
        } else {
            Err(anyhow::anyhow!("No client available for leader {}", leader))
        }
    }

    pub async fn read(&mut self, offset: u64) -> Result<Option<Vec<u8>>> {
        // Try to find the leader
        let leader = self.find_leader().await?;
        
        let mut clients = self.clients.write().await;
        if let Some(client) = clients.get_mut(&leader) {
            match client.read(offset).await {
                Ok(data) => Ok(data),
                Err(e) => {
                    error!("Failed to read from leader {}: {}", leader, e);
                    // Try to find a new leader
                    self.current_leader.write().await.take();
                    Err(e)
                }
            }
        } else {
            Err(anyhow::anyhow!("No client available for leader {}", leader))
        }
    }

    async fn find_leader(&self) -> Result<String> {
        // Check if we have a cached leader
        if let Some(leader) = self.current_leader.read().await.as_ref() {
            return Ok(leader.clone());
        }

        // Try to find the leader by trying all nodes
        let clients = self.clients.read().await;
        for (node_id, client) in clients.iter() {
            // Try a simple read operation to see if this node is the leader
            // In a real implementation, you'd have a specific RPC for this
            let mut client_clone = client.clone();
            if let Ok(_) = client_clone.read(0).await {
                // This node responded, assume it's the leader for now
                let mut current_leader = self.current_leader.write().await;
                *current_leader = Some(node_id.clone());
                return Ok(node_id.clone());
            }
        }

        Err(anyhow::anyhow!("No leader found"))
    }

    pub async fn get_cluster_info(&self) -> Result<Vec<String>> {
        let clients = self.clients.read().await;
        Ok(clients.keys().cloned().collect())
    }

    pub async fn disconnect_from_node(&mut self, node_id: &str) -> Result<()> {
        let mut clients = self.clients.write().await;
        clients.remove(node_id);
        
        // Clear leader cache if this was the leader
        let mut current_leader = self.current_leader.write().await;
        if current_leader.as_ref() == Some(&node_id.to_string()) {
            *current_leader = None;
        }
        
        info!("Disconnected from node {}", node_id);
        Ok(())
    }
}
