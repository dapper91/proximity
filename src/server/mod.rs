use std::fmt;
use std::io;
use std::net;

use async_trait::async_trait;

use crate::signal;

pub mod tcp;
pub mod udp;
pub mod unix;

pub use tcp::TCPServer;
pub use udp::UDPServer;
pub use unix::UnixServer;

pub enum ServerProto {
    TCP,
    UDP,
    UNIX,
}

impl fmt::Display for ServerProto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ServerProto::TCP => write!(f, "tcp"),
            ServerProto::UDP => write!(f, "udp"),
            ServerProto::UNIX => write!(f, "unix"),
        }
    }
}

pub enum ServerError {
    BindError(io::Error),
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ServerError::BindError(err) => write!(f, "server bind error: {}", err),
        }
    }
}

#[async_trait]
pub trait Server: Send {
    fn get_port(&self) -> u16;
    fn get_host(&self) -> net::IpAddr;
    fn get_proto(&self) -> ServerProto;

    async fn start(&self, mut sig_receiver: signal::Receiver) -> Result<(), ServerError>;
}
