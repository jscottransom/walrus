use tonic::{transport::Server, Request, Response, Status};

use ::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};

pub mod hello_world {
    tonic::include_proto!("log.v1"); // The string specified here must match the proto package name
}

