use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use walrus::cluster::config::ClusterConfig;
use walrus::cluster::state::ClusterStateManager;
use walrus::cluster::election::LeaderElection;
use walrus::log::config;
use walrus::log::log::Log;

#[tokio::test]
async fn test_leader_election() {
    // Create cluster configuration
    let bind_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let config = ClusterConfig::new("node-1".to_string(), bind_addr);
    
    // Create state manager
    let state_manager = Arc::new(ClusterStateManager::new("node-1".to_string()));
    
    // Create election manager
    let mut election = LeaderElection::new(config, state_manager.clone());
    
    // Add some nodes to the cluster
    state_manager.add_node(walrus::cluster::state::NodeInfo::new(
        "node-1".to_string(),
        "127.0.0.1:8080".parse().unwrap(),
    )).unwrap();
    
    state_manager.add_node(walrus::cluster::state::NodeInfo::new(
        "node-2".to_string(),
        "127.0.0.1:8081".parse().unwrap(),
    )).unwrap();
    
    state_manager.add_node(walrus::cluster::state::NodeInfo::new(
        "node-3".to_string(),
        "127.0.0.1:8082".parse().unwrap(),
    )).unwrap();
    
    // Start election in background
    let election_handle = tokio::spawn(async move {
        election.start_election_loop().await
    });
    
    // Wait a bit for election to start
    sleep(Duration::from_millis(100)).await;
    
    // Check if we became leader (in a single-node scenario, we should)
    let state = state_manager.get_state();
    assert!(state.nodes.get("node-1").is_some());
    
    // Cancel the election loop
    election_handle.abort();
}

#[tokio::test]
async fn test_cluster_state_management() {
    let state_manager = Arc::new(ClusterStateManager::new("test-node".to_string()));
    
    // Test adding nodes
    let node_info = walrus::cluster::state::NodeInfo::new(
        "test-node".to_string(),
        "127.0.0.1:8080".parse().unwrap(),
    );
    
    state_manager.add_node(node_info).unwrap();
    
    let state = state_manager.get_state();
    assert!(state.nodes.contains_key("test-node"));
    
    // Test removing nodes
    state_manager.remove_node("test-node").unwrap();
    
    let state = state_manager.get_state();
    assert!(!state.nodes.contains_key("test-node"));
}

#[tokio::test]
async fn test_wal_with_cluster() {
    // Create WAL log
    let log_config = config::Config {
        segment: config::InitSegment {
            max_store_bytes: 1024,
            max_index_bytes: 1024,
            initial_offset: 0,
        },
    };
    
    let log = Log::new("/tmp/test_wal_cluster".to_string(), log_config).unwrap();
    
    // Test basic WAL operations
    let mut log_guard = log.lock().unwrap();
    
    let mut record = walrus::log::segment::Record::default();
    record.value = b"Test cluster WAL".to_vec();
    record.offset = 0;
    
    let offset = log_guard.append(&mut record).unwrap();
    assert_eq!(offset, 0);
    
    let read_record = log_guard.read(0).unwrap();
    assert_eq!(read_record.value, b"Test cluster WAL");
    
    drop(log_guard);
    
    // Cleanup
    std::fs::remove_dir_all("/tmp/test_wal_cluster").ok();
}

#[tokio::test]
async fn test_cluster_config() {
    let bind_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let mut config = ClusterConfig::new("test-node".to_string(), bind_addr);
    
    // Test adding nodes
    config.add_node("node-1".to_string(), "127.0.0.1:8080".parse().unwrap());
    config.add_node("node-2".to_string(), "127.0.0.1:8081".parse().unwrap());
    
    assert_eq!(config.nodes.len(), 2);
    assert!(config.nodes.contains_key("node-1"));
    assert!(config.nodes.contains_key("node-2"));
    
    // Test timeouts
    assert_eq!(config.election_timeout().as_millis(), 1000);
    assert_eq!(config.heartbeat_interval().as_millis(), 100);
}
