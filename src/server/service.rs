use crate::cluster::state::ClusterStateManager;
use crate::cluster::config::ClusterConfig;
use crate::cluster::election::LeaderElection;
use crate::cluster::replication::ReplicationManager;
use crate::cluster::discovery::DiscoveryManager;
use crate::log::log::SafeLog;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

pub struct WalService {
    log: SafeLog,
    state_manager: Arc<ClusterStateManager>,
    config: ClusterConfig,
    election: LeaderElection,
    replication: ReplicationManager,
    discovery: DiscoveryManager,
}

impl WalService {
    pub fn new(
        log: SafeLog,
        state_manager: Arc<ClusterStateManager>,
        config: ClusterConfig,
    ) -> Self {
        let replication = ReplicationManager::new(config.clone(), state_manager.clone(), log.clone().into());
        let election = LeaderElection::new(config.clone(), state_manager.clone());
        let discovery = DiscoveryManager::new(config.clone(), state_manager.clone());

        Self {
            log,
            state_manager,
            config,
            election,
            replication,
            discovery,
        }
    }

    pub async fn start(&self) -> Result<()> {
        info!("Starting WAL service for node {}", self.config.node_id);

        // Start discovery
        let discovery_handle = {
            let discovery = self.discovery.clone();
            tokio::spawn(async move {
                if let Err(e) = discovery.start_discovery().await {
                    error!("Discovery failed: {}", e);
                }
            })
        };

        // Start election loop
        let election_handle = {
            let mut election = self.election.clone();
            tokio::spawn(async move {
                if let Err(e) = election.start_election_loop().await {
                    error!("Election loop failed: {}", e);
                }
            })
        };

        // Wait for both to complete (they shouldn't unless there's an error)
        tokio::select! {
            _ = discovery_handle => {
                error!("Discovery task completed unexpectedly");
            }
            _ = election_handle => {
                error!("Election task completed unexpectedly");
            }
        }

        Ok(())
    }

    pub async fn write(&self, data: Vec<u8>) -> Result<u64> {
        if !self.state_manager.is_leader() {
            return Err(anyhow::anyhow!("Not the leader"));
        }

        self.replication.append_entry(data).await
    }

    pub async fn read(&self, offset: u64) -> Result<Option<Vec<u8>>> {
        self.replication.read_entry(offset).await
    }

    pub fn is_leader(&self) -> bool {
        self.state_manager.is_leader()
    }

    pub async fn get_cluster_state(&self) -> crate::cluster::state::ClusterState {
        self.state_manager.get_state()
    }

    pub async fn get_alive_nodes(&self) -> Vec<String> {
        self.state_manager.get_alive_nodes()
    }
}

impl Clone for WalService {
    fn clone(&self) -> Self {
        Self {
            log: self.log.clone(),
            state_manager: self.state_manager.clone(),
            config: self.config.clone(),
            election: self.election.clone(),
            replication: self.replication.clone(),
            discovery: self.discovery.clone(),
        }
    }
}
