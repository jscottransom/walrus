use crate::cluster::state::{ClusterStateManager, NodeRole};
use crate::cluster::config::ClusterConfig;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct ElectionRequest {
    pub term: u64,
    pub candidate_id: String,
    pub last_log_index: u64,
    pub last_log_term: u64,
}

#[derive(Debug, Clone)]
pub struct ElectionResponse {
    pub term: u64,
    pub vote_granted: bool,
}

#[derive(Debug, Clone)]
pub struct HeartbeatRequest {
    pub term: u64,
    pub leader_id: String,
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub entries: Vec<LogEntry>,
    pub leader_commit: u64,
}

#[derive(Debug, Clone)]
pub struct HeartbeatResponse {
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
pub struct LeaderElection {
    config: ClusterConfig,
    state_manager: Arc<ClusterStateManager>,
    election_timeout: Duration,
    heartbeat_interval: Duration,
    last_heartbeat: Instant,
    election_timer: Instant,
}

impl LeaderElection {
    pub fn new(config: ClusterConfig, state_manager: Arc<ClusterStateManager>) -> Self {
        Self {
            election_timeout: config.election_timeout(),
            heartbeat_interval: config.heartbeat_interval(),
            last_heartbeat: Instant::now(),
            election_timer: Instant::now(),
            config,
            state_manager,
        }
    }

    pub async fn start_election_loop(&mut self) -> Result<()> {
        info!("Starting leader election loop for node {}", self.config.node_id);
        
        loop {
            let state = self.state_manager.get_state();
            
            match state.nodes.get(&self.config.node_id) {
                Some(node) => {
                    match node.role {
                        NodeRole::Follower => {
                            self.run_follower_loop().await?;
                        }
                        NodeRole::Candidate => {
                            self.run_candidate_loop().await?;
                        }
                        NodeRole::Leader => {
                            self.run_leader_loop().await?;
                        }
                    }
                }
                None => {
                    error!("Node {} not found in cluster state", self.config.node_id);
                    sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    async fn run_follower_loop(&mut self) -> Result<()> {
        debug!("Running follower loop");
        
        while !self.should_start_election() {
            sleep(Duration::from_millis(10)).await;
        }
        
        info!("Starting election as candidate");
        self.start_election().await?;
        Ok(())
    }

    async fn run_candidate_loop(&mut self) -> Result<()> {
        debug!("Running candidate loop");
        
        // Start election
        let votes = self.request_votes().await?;
        
        if votes >= self.state_manager.get_quorum_size() {
            info!("Won election with {} votes", votes);
            self.become_leader().await?;
        } else {
            warn!("Lost election with {} votes", votes);
            self.become_follower().await?;
        }
        
        Ok(())
    }

    async fn run_leader_loop(&mut self) -> Result<()> {
        debug!("Running leader loop");
        
        // Send heartbeats to all followers
        self.send_heartbeats().await?;
        
        // Wait for next heartbeat interval
        sleep(self.heartbeat_interval).await;
        
        Ok(())
    }

    fn should_start_election(&self) -> bool {
        let elapsed = self.election_timer.elapsed();
        elapsed >= self.election_timeout
    }

    async fn start_election(&mut self) -> Result<()> {
        // Increment term
        let new_term = self.state_manager.increment_term()?;
        
        // Vote for self
        self.state_manager.update_state(|state| {
            state.voted_for = Some(self.config.node_id.clone());
        })?;
        
        // Set role to candidate
        self.state_manager.set_role(&self.config.node_id, NodeRole::Candidate)?;
        
        // Reset election timer
        self.election_timer = Instant::now();
        
        info!("Started election for term {}", new_term);
        Ok(())
    }

    async fn request_votes(&self) -> Result<usize> {
        let state = self.state_manager.get_state();
        let mut votes = 1; // Vote for self
        
        let request = ElectionRequest {
            term: state.current_term,
            candidate_id: self.config.node_id.clone(),
            last_log_index: state.last_applied,
            last_log_term: state.current_term,
        };
        
        // Request votes from all other nodes
        for (node_id, node_info) in &state.nodes {
            if node_id == &self.config.node_id {
                continue;
            }
            
            if let Ok(response) = self.send_vote_request(node_id, &request).await {
                if response.vote_granted {
                    votes += 1;
                }
            }
        }
        
        Ok(votes)
    }

    async fn send_vote_request(&self, node_id: &str, request: &ElectionRequest) -> Result<ElectionResponse> {
        // This would typically use gRPC to send the request
        // For now, we'll simulate the response
        let state = self.state_manager.get_state();
        
        if let Some(node) = state.nodes.get(node_id) {
            if request.term >= node.term {
                Ok(ElectionResponse {
                    term: request.term,
                    vote_granted: true,
                })
            } else {
                Ok(ElectionResponse {
                    term: node.term,
                    vote_granted: false,
                })
            }
        } else {
            Err(anyhow::anyhow!("Node {} not found", node_id))
        }
    }

    async fn become_leader(&mut self) -> Result<()> {
        self.state_manager.set_leader(self.config.node_id.clone())?;
        self.state_manager.set_role(&self.config.node_id, NodeRole::Leader)?;
        
        info!("Became leader for term {}", self.state_manager.get_state().current_term);
        Ok(())
    }

    async fn become_follower(&mut self) -> Result<()> {
        self.state_manager.set_role(&self.config.node_id, NodeRole::Follower)?;
        self.election_timer = Instant::now();
        
        debug!("Became follower");
        Ok(())
    }

    async fn send_heartbeats(&self) -> Result<()> {
        let state = self.state_manager.get_state();
        
        let heartbeat = HeartbeatRequest {
            term: state.current_term,
            leader_id: self.config.node_id.clone(),
            prev_log_index: state.last_applied,
            prev_log_term: state.current_term,
            entries: vec![],
            leader_commit: state.commit_index,
        };
        
        for (node_id, node_info) in &state.nodes {
            if node_id == &self.config.node_id {
                continue;
            }
            
            if let Err(e) = self.send_heartbeat(node_id, &heartbeat).await {
                warn!("Failed to send heartbeat to {}: {}", node_id, e);
            }
        }
        
        Ok(())
    }

    async fn send_heartbeat(&self, node_id: &str, heartbeat: &HeartbeatRequest) -> Result<HeartbeatResponse> {
        // This would typically use gRPC to send the heartbeat
        // For now, we'll simulate the response
        let state = self.state_manager.get_state();
        
        if let Some(node) = state.nodes.get(node_id) {
            if heartbeat.term >= node.term {
                // Update node's term and mark as alive
                self.state_manager.update_heartbeat(node_id)?;
                
                Ok(HeartbeatResponse {
                    term: heartbeat.term,
                    success: true,
                    match_index: state.last_applied,
                })
            } else {
                Ok(HeartbeatResponse {
                    term: node.term,
                    success: false,
                    match_index: 0,
                })
            }
        } else {
            Err(anyhow::anyhow!("Node {} not found", node_id))
        }
    }

    pub async fn handle_vote_request(&self, request: ElectionRequest) -> Result<ElectionResponse> {
        let mut state = self.state_manager.get_state();
        
        if request.term < state.current_term {
            return Ok(ElectionResponse {
                term: state.current_term,
                vote_granted: false,
            });
        }
        
        if request.term > state.current_term {
            state.current_term = request.term;
            state.voted_for = None;
            state.leader_id = None;
        }
        
        let vote_granted = state.voted_for.is_none() || state.voted_for == Some(request.candidate_id.clone());
        
        if vote_granted {
            state.voted_for = Some(request.candidate_id);
            self.state_manager.update_state(|s| {
                s.current_term = state.current_term;
                s.voted_for = state.voted_for.clone();
                s.leader_id = state.leader_id.clone();
            })?;
        }
        
        Ok(ElectionResponse {
            term: state.current_term,
            vote_granted,
        })
    }

    pub async fn handle_heartbeat(&mut self, heartbeat: HeartbeatRequest) -> Result<HeartbeatResponse> {
        let mut state = self.state_manager.get_state();
        
        if heartbeat.term < state.current_term {
            return Ok(HeartbeatResponse {
                term: state.current_term,
                success: false,
                match_index: 0,
            });
        }
        
        if heartbeat.term > state.current_term {
            state.current_term = heartbeat.term;
            state.voted_for = None;
        }
        
        state.leader_id = Some(heartbeat.leader_id.clone());
        self.state_manager.update_state(|s| {
            s.current_term = state.current_term;
            s.voted_for = state.voted_for.clone();
            s.leader_id = state.leader_id.clone();
        })?;
        
        // Reset election timer
        self.election_timer = Instant::now();
        
        Ok(HeartbeatResponse {
            term: state.current_term,
            success: true,
            match_index: state.last_applied,
        })
    }
}
