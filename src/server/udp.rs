use std::net;
use std::sync;

use async_trait::async_trait;
use log::Log;

use super::{Server, ServerError, ServerProto};
use crate::signal::{self, Signal};
use crate::upstream::Upstream;

const UDP_PACKET_MAX_SIZE: usize = 65535;

pub struct UDPServer {
    host: net::IpAddr,
    port: u16,
    upstream: sync::Arc<tokio::sync::RwLock<Box<dyn Upstream>>>,
    logger: Box<dyn Log>,
}

impl UDPServer {
    pub fn new(
        host: net::IpAddr,
        port: u16,
        upstream: sync::Arc<tokio::sync::RwLock<Box<dyn Upstream>>>,
        logger: Box<dyn Log>,
    ) -> Self {
        UDPServer {
            host,
            port,
            upstream,
            logger,
        }
    }

    async fn handle_connection(&self, data: Box<[u8]>, address: net::SocketAddr) {
        unimplemented!()
    }
}

#[async_trait]
impl Server for UDPServer {
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
        let mut buf = [0; UDP_PACKET_MAX_SIZE];

        let address = net::SocketAddr::new(self.host, self.port);
        let listener = tokio::net::UdpSocket::bind(address).await.unwrap();

        loop {
            tokio::select! {
                sig = sig_receiver.receive() => {
                    match sig {
                        Signal::Stop => { break },
                        _ => { unimplemented!() },
                    }
                },
                result = listener.recv_from(&mut buf) => {
                    let (len, addr) = result.unwrap();
                    self.handle_connection(buf[0..len].into(), addr).await;
                },
            }
        }

        return Ok(());
    }
}
