# Walrus - Distributed Write-Ahead Log

A distributed Write-Ahead Log (WAL) system built in Rust with leader election, replication, and Kubernetes deployment support.

## Features

- **Distributed Consensus**: Implements Raft consensus algorithm for leader election and log replication
- **Fault Tolerance**: Automatic leader election and recovery from node failures
- **High Availability**: Multi-node cluster with quorum-based consistency
- **Persistent Storage**: Segmented log storage with configurable segment sizes
- **gRPC Interface**: Modern API for client-server communication
- **Kubernetes Ready**: Helm chart for easy deployment as StatefulSet
- **Observability**: Comprehensive logging and metrics

## Architecture

### Cluster Components

1. **Leader Election**: Raft-based consensus for selecting cluster leader
2. **Replication**: Master-slave replication with consistency guarantees
3. **Discovery**: Automatic node discovery and health monitoring
4. **State Management**: Distributed cluster state tracking

### Data Flow

```
Client Request → Leader Node → Log Append → Replicate to Followers → Commit
```

## Quick Start

### Local Development

1. **Build the project**:
   ```bash
   cargo build --release
   ```

2. **Start a single node**:
   ```bash
   ./target/release/walrus --node-id node-1 --bind-addr 127.0.0.1:8080
   ```

3. **Start additional nodes** (in separate terminals):
   ```bash
   ./target/release/walrus --node-id node-2 --bind-addr 127.0.0.1:8081
   ./target/release/walrus --node-id node-3 --bind-addr 127.0.0.1:8082
   ```

### Kubernetes Deployment

1. **Build and push the Docker image**:
   ```bash
   docker build -t your-registry/walrus:0.1.0 .
   docker push your-registry/walrus:0.1.0
   ```

2. **Deploy using Helm**:
   ```bash
   helm install walrus ./helm/walrus \
     --set image.repository=your-registry/walrus \
     --set image.tag=0.1.0
   ```

3. **Scale the cluster**:
   ```bash
   kubectl scale statefulset walrus --replicas=5
   ```

## Configuration

### Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `--node-id` | Unique node identifier | `node-1` |
| `--bind-addr` | Network address to bind | `127.0.0.1:8080` |
| `--data-dir` | Data storage directory | `/tmp/walrus` |
| `--max-segment-bytes` | Maximum segment size | `1048576` (1MB) |
| `--max-index-bytes` | Maximum index size | `1048576` (1MB) |
| `--election-timeout-ms` | Leader election timeout | `1000` |
| `--heartbeat-interval-ms` | Heartbeat interval | `100` |

### Helm Values

The Helm chart supports customization:

```yaml
replicaCount: 3
image:
  repository: your-registry/walrus
  tag: "0.1.0"

config:
  electionTimeoutMs: 1000
  heartbeatIntervalMs: 100
  maxSegmentBytes: 1048576
  maxIndexBytes: 1048576
  dataDir: "/data"

persistence:
  enabled: true
  size: 10Gi
```

## API Usage

### gRPC Client Example

```rust
use walrus::client::WalClient;
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = "127.0.0.1:8080".parse()?;
    let mut client = WalClient::new(addr).await?;
    
    // Write data
    let data = b"Hello, distributed WAL!".to_vec();
    let offset = client.write(data, 0).await?;
    println!("Written at offset: {}", offset);
    
    // Read data
    if let Some(data) = client.read(offset).await? {
        println!("Read: {}", String::from_utf8_lossy(&data));
    }
    
    Ok(())
}
```

## Cluster Management

### Leader Election

The system uses the Raft consensus algorithm for leader election:

1. **Follower State**: Nodes start as followers, waiting for leader heartbeats
2. **Candidate State**: If no heartbeat received, node becomes candidate and requests votes
3. **Leader State**: Node with majority votes becomes leader and starts sending heartbeats

### Replication

- **Log Entries**: All writes go through the leader
- **Quorum Commitment**: Entries are committed when replicated to majority
- **Consistency**: Strong consistency guarantees across the cluster

### Failure Recovery

- **Automatic Detection**: Dead nodes detected via heartbeat timeouts
- **Leader Failover**: New leader elected when current leader fails
- **Data Recovery**: Followers replay log entries from leader

## Monitoring

### Health Checks

The service provides health endpoints:

- `/health` - Liveness probe
- `/ready` - Readiness probe

### Metrics

Key metrics to monitor:

- **Cluster State**: Leader/follower status
- **Replication Lag**: Time between leader and follower sync
- **Election Frequency**: Leader election events
- **Write Throughput**: Records per second
- **Storage Usage**: Segment and index sizes

## Development

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with specific configuration
cargo run -- --node-id test-node --bind-addr 127.0.0.1:8080
```

### Testing

```bash
# Run unit tests
cargo test

# Run integration tests
cargo test --test wal_test

# Run with coverage
cargo tarpaulin
```






