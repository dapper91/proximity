use std::net;
use std::sync;

use async_trait::async_trait;
use log::Log;
use tokio;

use super::{Server, ServerError, ServerProto};
use crate::signal::{self, Signal};
use crate::upstream::Upstream;

pub struct TCPServer {
    host: net::IpAddr,
    port: u16,
    upstream: sync::Arc<tokio::sync::RwLock<Box<dyn Upstream>>>,

    logger: sync::Arc<dyn Log>,
}

impl TCPServer {
    pub fn new(
        host: net::IpAddr,
        port: u16,
        upstream: sync::Arc<tokio::sync::RwLock<Box<dyn Upstream>>>,
        logger: Box<dyn Log>,
    ) -> Self {
        TCPServer {
            host,
            port,
            upstream,
            logger: logger.into(),
        }
    }

    async fn handle_connection(
        upstream: sync::Arc<tokio::sync::RwLock<Box<dyn Upstream>>>,
        logger: sync::Arc<dyn Log>,
        socket: tokio::net::TcpStream,
        address: net::SocketAddr,
    ) {
        unimplemented!();
        // let upstream = upstream.read().await;
        // let host = upstream.start_session()
        // tokio::net::TcpSocket::connect()
    }
}

#[async_trait]
impl Server for TCPServer {
    fn get_port(&self) -> u16 {
        self.port
    }

    fn get_host(&self) -> net::IpAddr {
        self.host
    }

    fn get_proto(&self) -> ServerProto {
        ServerProto::TCP
    }

    async fn start(&self, mut sig_receiver: signal::Receiver) -> Result<(), ServerError> {
        let address = net::SocketAddr::new(self.host, self.port);

        let listener = tokio::net::TcpListener::bind(address)
            .await
            .map_err(|err| ServerError::BindError(err))?;

        // let task_handles = vec![];
        loop {
            tokio::select! {
                sig = sig_receiver.receive() => {
                    match sig {
                        Signal::Stop => { break },
                        _ => { unimplemented!() },
                    }
                },
                result = listener.accept() => {
                    match result {
                        Ok((socket, addr)) => {
                             let handle = tokio::spawn(
                                TCPServer::handle_connection(self.upstream.clone(), self.logger.clone(), socket, addr)
                             );
                        },
                        Err(err) => {
                            log::warn!("connection accepting error: {}", err);
                        },
                    }
                },
            }
        }

        return Ok(());
    }
}
