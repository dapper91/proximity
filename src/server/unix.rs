use std::net;
use std::sync;

use async_trait::async_trait;
use log::Log;

use super::{Server, ServerError, ServerProto};
use crate::signal;
use crate::upstream::Upstream;

pub struct UnixServer {
    host: net::IpAddr,
    port: u16,
    upstream: sync::Arc<tokio::sync::RwLock<Box<dyn Upstream>>>,
    logger: Box<dyn Log>,
}

impl UnixServer {
    pub fn new(
        host: net::IpAddr,
        port: u16,
        upstream: sync::Arc<tokio::sync::RwLock<Box<dyn Upstream>>>,
        logger: Box<dyn Log>,
    ) -> Self {
        UnixServer {
            host,
            port,
            upstream,
            logger,
        }
    }
}

#[async_trait]
impl Server for UnixServer {
    fn get_port(&self) -> u16 {
        self.port
    }

    fn get_host(&self) -> net::IpAddr {
        self.host
    }

    fn get_proto(&self) -> ServerProto {
        ServerProto::TCP
    }

    async fn start(&self, sig_receiver: signal::Receiver) -> Result<(), ServerError> {
        unimplemented!();
    }
}
