use crate::cluster::state::{ClusterStateManager, NodeRole};
use crate::cluster::config::ClusterConfig;
use crate::log::segment::Record;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct ReplicationRequest {
    pub term: u64,
    pub leader_id: String,
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub entries: Vec<LogEntry>,
    pub leader_commit: u64,
}

#[derive(Debug, Clone)]
pub struct ReplicationResponse {
    pub term: u64,
    pub success: bool,
    pub match_index: u64,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub term: u64,
    pub index: u64,
    pub command: Vec<u8>,
}

#[derive(Clone)]
pub struct ReplicationManager {
    config: ClusterConfig,
    state_manager: Arc<ClusterStateManager>,
    next_index: Arc<RwLock<HashMap<String, u64>>>,
    match_index: Arc<RwLock<HashMap<String, u64>>>,
    log: Arc<crate::log::log::SafeLog>,
}

impl ReplicationManager {
    pub fn new(
        config: ClusterConfig,
        state_manager: Arc<ClusterStateManager>,
        log: Arc<crate::log::log::SafeLog>,
    ) -> Self {
        Self {
            config,
            state_manager,
            next_index: Arc::new(RwLock::new(HashMap::new())),
            match_index: Arc::new(RwLock::new(HashMap::new())),
            log,
        }
    }

    pub async fn replicate_to_followers(&self, entries: Vec<LogEntry>) -> Result<bool> {
        if !self.state_manager.is_leader() {
            return Err(anyhow::anyhow!("Not the leader"));
        }

        let state = self.state_manager.get_state();
        let alive_nodes = self.state_manager.get_alive_nodes();
        let mut success_count = 0;

        for node_id in alive_nodes {
            if node_id == self.config.node_id {
                continue;
            }

            if let Ok(true) = self.replicate_to_node(&node_id, &entries).await {
                success_count += 1;
            }
        }

        // Check if we have a quorum
        let quorum_size = self.state_manager.get_quorum_size();
        let success = success_count >= quorum_size - 1; // -1 because leader doesn't count

        if success {
            // Update commit index
            self.update_commit_index().await?;
        }

        Ok(success)
    }

    async fn replicate_to_node(&self, node_id: &str, entries: &[LogEntry]) -> Result<bool> {
        let mut next_index = self.next_index.write().await;
        let mut match_index = self.match_index.write().await;

        let next_idx = next_index.get(node_id).copied().unwrap_or(0);
        
        if entries.is_empty() {
            // This is just a heartbeat
            let request = ReplicationRequest {
                term: self.state_manager.get_state().current_term,
                leader_id: self.config.node_id.clone(),
                prev_log_index: next_idx.saturating_sub(1),
                prev_log_term: 0, // We'd need to track this properly
                entries: vec![],
                leader_commit: self.state_manager.get_state().commit_index,
            };

            if let Ok(response) = self.send_replication_request(node_id, &request).await {
                if response.success {
                    match_index.insert(node_id.to_string(), response.match_index);
                    return Ok(true);
                } else {
                    // Decrement next_index and retry
                    let new_next = next_idx.saturating_sub(1);
                    next_index.insert(node_id.to_string(), new_next);
                    return Ok(false);
                }
            }
        } else {
            // Send actual log entries
            let request = ReplicationRequest {
                term: self.state_manager.get_state().current_term,
                leader_id: self.config.node_id.clone(),
                prev_log_index: next_idx.saturating_sub(1),
                prev_log_term: 0, // We'd need to track this properly
                entries: entries.to_vec(),
                leader_commit: self.state_manager.get_state().commit_index,
            };

            if let Ok(response) = self.send_replication_request(node_id, &request).await {
                if response.success {
                    let new_next = next_idx + entries.len() as u64;
                    next_index.insert(node_id.to_string(), new_next);
                    match_index.insert(node_id.to_string(), response.match_index);
                    return Ok(true);
                } else {
                    // Decrement next_index and retry
                    let new_next = next_idx.saturating_sub(1);
                    next_index.insert(node_id.to_string(), new_next);
                    return Ok(false);
                }
            }
        }

        Ok(false)
    }

    async fn send_replication_request(&self, node_id: &str, request: &ReplicationRequest) -> Result<ReplicationResponse> {
        // This would typically use gRPC to send the request
        // For now, we'll simulate the response
        let state = self.state_manager.get_state();
        
        if let Some(node) = state.nodes.get(node_id) {
            if request.term >= node.term {
                // Simulate successful replication
                Ok(ReplicationResponse {
                    term: request.term,
                    success: true,
                    match_index: request.prev_log_index + request.entries.len() as u64,
                })
            } else {
                Ok(ReplicationResponse {
                    term: node.term,
                    success: false,
                    match_index: 0,
                })
            }
        } else {
            Err(anyhow::anyhow!("Node {} not found", node_id))
        }
    }

    async fn update_commit_index(&self) -> Result<()> {
        let match_indices = self.match_index.read().await;
        let mut indices: Vec<u64> = match_indices.values().copied().collect();
        indices.sort();

        // Find the median index that has been replicated to a majority
        let quorum_size = self.state_manager.get_quorum_size();
        if indices.len() >= quorum_size - 1 { // -1 because leader doesn't count
            let median_index = indices[indices.len() - quorum_size + 1];
            
            self.state_manager.update_state(|state| {
                state.commit_index = median_index;
            })?;
        }

        Ok(())
    }

    pub async fn handle_replication_request(&self, request: ReplicationRequest) -> Result<ReplicationResponse> {
        let state = self.state_manager.get_state();
        
        if request.term < state.current_term {
            return Ok(ReplicationResponse {
                term: state.current_term,
                success: false,
                match_index: 0,
            });
        }

        // Apply log entries
        for entry in &request.entries {
            let mut record = Record::default();
            record.value = entry.command.clone();
            record.offset = entry.index;
            
            let mut log_guard = self.log.lock().unwrap();
            if let Err(e) = log_guard.append(&mut record) {
                warn!("Failed to append log entry: {}", e);
                return Ok(ReplicationResponse {
                    term: state.current_term,
                    success: false,
                    match_index: 0,
                });
            }
        }

        // Update commit index
        if request.leader_commit > state.commit_index {
            self.state_manager.update_state(|s| {
                s.commit_index = std::cmp::min(request.leader_commit, s.last_applied);
            })?;
        }

        Ok(ReplicationResponse {
            term: state.current_term,
            success: true,
            match_index: request.prev_log_index + request.entries.len() as u64,
        })
    }

    pub async fn append_entry(&self, command: Vec<u8>) -> Result<u64> {
        if !self.state_manager.is_leader() {
            return Err(anyhow::anyhow!("Not the leader"));
        }

        let state = self.state_manager.get_state();
        let log_index = state.last_applied + 1;

        // Create log entry
        let entry = LogEntry {
            term: state.current_term,
            index: log_index,
            command,
        };

        // Append to local log
        let mut record = Record::default();
        record.value = entry.command.clone();
        record.offset = entry.index;
        
        let mut log_guard = self.log.lock().unwrap();
        let offset = log_guard.append(&mut record).map_err(|e| anyhow::anyhow!("Failed to append record: {}", e))?;
        drop(log_guard);

        // Update last applied
        self.state_manager.update_state(|s| {
            s.last_applied = log_index;
        })?;

        // Replicate to followers
        if let Ok(true) = self.replicate_to_followers(vec![entry]).await {
            info!("Successfully replicated entry {} to quorum", log_index);
        } else {
            warn!("Failed to replicate entry {} to quorum", log_index);
        }

        Ok(offset)
    }

    pub async fn read_entry(&self, index: u64) -> Result<Option<Vec<u8>>> {
        let mut log_guard = self.log.lock().unwrap();
        
        match log_guard.read(index) {
            Ok(record) => Ok(Some(record.value)),
            Err(_) => Ok(None),
        }
    }

    pub async fn get_commit_index(&self) -> u64 {
        self.state_manager.get_state().commit_index
    }

    pub async fn get_last_applied(&self) -> u64 {
        self.state_manager.get_state().last_applied
    }
}
