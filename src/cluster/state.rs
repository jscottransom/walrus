use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeRole {
    Follower,
    Candidate,
    Leader,
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: String,
    pub addr: SocketAddr,
    pub role: NodeRole,
    pub term: u64,
    pub last_heartbeat: Option<Instant>,
    pub is_alive: bool,
}

impl NodeInfo {
    pub fn new(id: String, addr: SocketAddr) -> Self {
        Self {
            id,
            addr,
            role: NodeRole::Follower,
            term: 0,
            last_heartbeat: None,
            is_alive: true,
        }
    }

    pub fn is_leader(&self) -> bool {
        matches!(self.role, NodeRole::Leader)
    }

    pub fn is_follower(&self) -> bool {
        matches!(self.role, NodeRole::Follower)
    }

    pub fn is_candidate(&self) -> bool {
        matches!(self.role, NodeRole::Candidate)
    }
}

#[derive(Debug, Clone)]
pub struct ClusterState {
    pub current_term: u64,
    pub voted_for: Option<String>,
    pub leader_id: Option<String>,
    pub nodes: HashMap<String, NodeInfo>,
    pub commit_index: u64,
    pub last_applied: u64,
}

impl Default for ClusterState {
    fn default() -> Self {
        Self {
            current_term: 0,
            voted_for: None,
            leader_id: None,
            nodes: HashMap::new(),
            commit_index: 0,
            last_applied: 0,
        }
    }
}

#[derive(Debug)]
pub struct ClusterStateManager {
    state: Arc<RwLock<ClusterState>>,
    node_id: String,
}

impl ClusterStateManager {
    pub fn new(node_id: String) -> Self {
        Self {
            state: Arc::new(RwLock::new(ClusterState::default())),
            node_id,
        }
    }

    pub fn get_state(&self) -> ClusterState {
        self.state.read().unwrap().clone()
    }

    pub fn update_state<F>(&self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut ClusterState),
    {
        let mut state = self.state.write().unwrap();
        f(&mut state);
        Ok(())
    }

    pub fn increment_term(&self) -> anyhow::Result<u64> {
        let mut state = self.state.write().unwrap();
        state.current_term += 1;
        state.voted_for = None;
        state.leader_id = None;
        Ok(state.current_term)
    }

    pub fn set_leader(&self, leader_id: String) -> anyhow::Result<()> {
        let mut state = self.state.write().unwrap();
        state.leader_id = Some(leader_id);
        Ok(())
    }

    pub fn set_role(&self, node_id: &str, role: NodeRole) -> anyhow::Result<()> {
        let mut state = self.state.write().unwrap();
        if let Some(node) = state.nodes.get_mut(node_id) {
            node.role = role;
        }
        Ok(())
    }

    pub fn add_node(&self, node_info: NodeInfo) -> anyhow::Result<()> {
        let mut state = self.state.write().unwrap();
        state.nodes.insert(node_info.id.clone(), node_info);
        Ok(())
    }

    pub fn remove_node(&self, node_id: &str) -> anyhow::Result<()> {
        let mut state = self.state.write().unwrap();
        state.nodes.remove(node_id);
        Ok(())
    }

    pub fn update_heartbeat(&self, node_id: &str) -> anyhow::Result<()> {
        let mut state = self.state.write().unwrap();
        if let Some(node) = state.nodes.get_mut(node_id) {
            node.last_heartbeat = Some(Instant::now());
            node.is_alive = true;
        }
        Ok(())
    }

    pub fn mark_node_dead(&self, node_id: &str) -> anyhow::Result<()> {
        let mut state = self.state.write().unwrap();
        if let Some(node) = state.nodes.get_mut(node_id) {
            node.is_alive = false;
        }
        Ok(())
    }

    pub fn get_leader(&self) -> Option<String> {
        self.state.read().unwrap().leader_id.clone()
    }

    pub fn is_leader(&self) -> bool {
        self.get_leader() == Some(self.node_id.clone())
    }

    pub fn get_alive_nodes(&self) -> Vec<String> {
        self.state
            .read()
            .unwrap()
            .nodes
            .iter()
            .filter(|(_, node)| node.is_alive)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn get_quorum_size(&self) -> usize {
        let alive_nodes = self.get_alive_nodes();
        (alive_nodes.len() / 2) + 1
    }
}
