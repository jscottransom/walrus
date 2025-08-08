use clap::Parser;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info, Level};
use tracing_subscriber;

use walrus::cluster::config::ClusterConfig;
use walrus::cluster::state::ClusterStateManager;
use walrus::log::config;
use walrus::log::log::Log;
use walrus::server::WalServer;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Node ID for this instance
    #[arg(short, long, default_value = "node-1")]
    node_id: String,

    /// Bind address for this node
    #[arg(short, long, default_value = "127.0.0.1:8080")]
    bind_addr: String,

    /// Data directory for WAL storage
    #[arg(short, long, default_value = "/tmp/walrus")]
    data_dir: String,

    /// Maximum segment size in bytes
    #[arg(long, default_value = "1048576")]
    max_segment_bytes: u64,

    /// Maximum index size in bytes
    #[arg(long, default_value = "1048576")]
    max_index_bytes: u64,

    /// Election timeout in milliseconds
    #[arg(long, default_value = "1000")]
    election_timeout_ms: u64,

    /// Heartbeat interval in milliseconds
    #[arg(long, default_value = "100")]
    heartbeat_interval_ms: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    // Parse command line arguments
    let args = Args::parse();

    info!("Starting distributed WAL server");
    info!("Node ID: {}", args.node_id);
    info!("Bind address: {}", args.bind_addr);
    info!("Data directory: {}", args.data_dir);

    // Parse bind address
    let bind_addr = SocketAddr::from_str(&args.bind_addr)?;

    // Create cluster configuration
    let mut cluster_config = ClusterConfig::new(args.node_id.clone(), bind_addr);
    cluster_config.data_dir = args.data_dir;
    cluster_config.max_segment_bytes = args.max_segment_bytes;
    cluster_config.max_index_bytes = args.max_index_bytes;
    cluster_config.election_timeout_ms = args.election_timeout_ms;
    cluster_config.heartbeat_interval_ms = args.heartbeat_interval_ms;

    // Create WAL log configuration
    let log_config = config::Config {
        segment: config::InitSegment {
            max_store_bytes: cluster_config.max_segment_bytes,
            max_index_bytes: cluster_config.max_index_bytes,
            initial_offset: 0,
        },
    };

    // Create WAL log
    let log = Log::new(cluster_config.data_dir.clone(), log_config).map_err(|e| anyhow::anyhow!("Failed to create log: {}", e))?;

    // Create cluster state manager
    let state_manager = Arc::new(ClusterStateManager::new(args.node_id.clone()));

    // Create WAL server
    let server = WalServer::new(log, state_manager, cluster_config);

    info!("Starting WAL server on {}", bind_addr);
    
    // Start the server
    server.start_server().await?;

    Ok(())
}
