use tonic::{transport::Server, Request, Response, Status};
use crate::cluster::state::ClusterStateManager;
use crate::cluster::config::ClusterConfig;
use crate::log::log::SafeLog;
use std::sync::Arc;
use tracing::{error, info};

// Import the generated protobuf code
pub mod proto {
    tonic::include_proto!("log");
}

use proto::log_server::{Log, LogServer};
use proto::{WriteRequest, WriteResponse, ReadRequest, ReadResponse, Record};

pub struct WalServer {
    log: SafeLog,
    state_manager: Arc<ClusterStateManager>,
    config: ClusterConfig,
}

impl WalServer {
    pub fn new(log: SafeLog, state_manager: Arc<ClusterStateManager>, config: ClusterConfig) -> Self {
        Self {
            log,
            state_manager,
            config,
        }
    }

    pub async fn start_server(self) -> anyhow::Result<()> {
        let addr = self.config.bind_addr;
        let svc = LogServer::new(self);
        
        info!("Starting WAL server on {}", addr);
        
        Server::builder()
            .add_service(svc)
            .serve(addr)
            .await?;
            
        Ok(())
    }
}

#[tonic::async_trait]
impl Log for WalServer {
    async fn write(
        &self,
        request: Request<WriteRequest>,
    ) -> Result<Response<WriteResponse>, Status> {
        let req = request.into_inner();
        
        // Check if we're the leader
        if !self.state_manager.is_leader() {
            return Err(Status::failed_precondition("Not the leader"));
        }
        
        // Extract the record
        let record = req.record.ok_or_else(|| Status::invalid_argument("No record provided"))?;
        
        // Append to log
        let mut log_guard = self.log.lock().unwrap();
        let mut wal_record = crate::log::segment::Record::default();
        wal_record.value = record.value;
        wal_record.offset = record.offset;
        
        match log_guard.append(&mut wal_record) {
            Ok(offset) => {
                info!("Successfully wrote record at offset {}", offset);
                Ok(Response::new(WriteResponse { offset }))
            }
            Err(e) => {
                error!("Failed to write record: {}", e);
                Err(Status::internal(format!("Failed to write record: {}", e)))
            }
        }
    }

    async fn read(
        &self,
        request: Request<ReadRequest>,
    ) -> Result<Response<ReadResponse>, Status> {
        let req = request.into_inner();
        let offset = req.offset;
        
        // Read from log
        let mut log_guard = self.log.lock().unwrap();
        
        match log_guard.read(offset) {
            Ok(record) => {
                let proto_record = Record {
                    value: record.value,
                    offset: record.offset,
                };
                
                Ok(Response::new(ReadResponse {
                    record: Some(proto_record),
                }))
            }
            Err(e) => {
                error!("Failed to read record at offset {}: {}", offset, e);
                Err(Status::not_found(format!("Record not found at offset {}: {}", offset, e)))
            }
        }
    }
}
