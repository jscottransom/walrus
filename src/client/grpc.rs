use tonic::{transport::Channel, Request};
use std::net::SocketAddr;
use anyhow::Result;

// Import the generated protobuf code
pub mod proto {
    tonic::include_proto!("log");
}

use proto::log_client::LogClient;
use proto::{WriteRequest, WriteResponse, ReadRequest, ReadResponse, Record};

#[derive(Clone)]
pub struct WalClient {
    client: LogClient<Channel>,
}

impl WalClient {
    pub async fn new(addr: SocketAddr) -> Result<Self> {
        let channel = Channel::from_shared(format!("http://{}", addr))?
            .connect()
            .await?;
        
        let client = LogClient::new(channel);
        
        Ok(Self { client })
    }

    pub async fn write(&mut self, data: Vec<u8>, offset: u64) -> Result<u64> {
        let record = Record {
            value: data,
            offset,
        };
        
        let request = Request::new(WriteRequest {
            record: Some(record),
        });
        
        let response = self.client.write(request).await?;
        Ok(response.into_inner().offset)
    }

    pub async fn read(&mut self, offset: u64) -> Result<Option<Vec<u8>>> {
        let request = Request::new(ReadRequest { offset });
        
        match self.client.read(request).await {
            Ok(response) => {
                let record = response.into_inner().record;
                Ok(record.map(|r| r.value))
            }
            Err(status) if status.code() == tonic::Code::NotFound => {
                Ok(None)
            }
            Err(e) => {
                Err(anyhow::anyhow!("Failed to read record: {}", e))
            }
        }
    }
}
