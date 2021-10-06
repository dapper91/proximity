use std::collections::BinaryHeap;
use std::error::Error;
use std::net;
use std::str::FromStr;
use std::sync::Arc;
use std::time;

use async_trait::async_trait;

pub mod sampler;

use crate::resolver::Resolver;
use sampler::Sampler;

#[derive(Debug, Clone)]
pub struct Host {
    hostname: String,
    port: u16,
    ipv6: bool,

    weight: u8,
    max_fails: u8,
    max_conns: u16,
    fail_timeout: time::Duration,

    fails: BinaryHeap<std::cmp::Reverse<time::Instant>>,
    last_fail: time::Instant,
}

impl PartialEq for Host {
    fn eq(&self, other: &Self) -> bool {
        self.hostname == other.hostname
            && self.port == other.port
            && self.weight == other.weight
            && self.max_fails == other.max_fails
            && self.fail_timeout == other.fail_timeout
    }
}

impl Host {
    pub fn new(hostname: &str, port: u16) -> Self {
        Host {
            hostname: hostname.into(),
            port,
            ipv6: false,
            weight: 1,
            max_fails: 1,
            max_conns: 1024,
            fail_timeout: time::Duration::from_secs(30),
            fails: BinaryHeap::new(),
            last_fail: time::Instant::now(),
        }
    }

    pub fn builder(hostname: &str, port: u16) -> HostBuilder {
        HostBuilder {
            host: Box::new(Self::new(hostname, port)),
        }
    }

    pub fn failed(&mut self) {
        let now = time::Instant::now();

        while let Some(fail) = self.fails.peek() {
            if fail.0 >= now - self.fail_timeout {
                break;
            }
            self.fails.pop();
        }
        self.fails.push(std::cmp::Reverse(now));
        self.last_fail = now;
    }
}

pub struct HostBuilder {
    host: Box<Host>,
}

impl HostBuilder {
    pub fn build(self) -> Host {
        *self.host
    }

    pub fn with_ipv6(mut self, value: bool) -> Self {
        self.host.ipv6 = value;
        self
    }

    pub fn with_weight(mut self, weight: u8) -> Self {
        self.host.weight = weight;
        self
    }

    pub fn with_max_fails(mut self, max_fails: u8) -> Self {
        self.host.max_fails = max_fails;
        self
    }

    pub fn with_max_conns(mut self, max_conns: u16) -> Self {
        self.host.max_conns = max_conns;
        self
    }

    pub fn with_fail_timeout(mut self, fail_timeout: time::Duration) -> Self {
        self.host.fail_timeout = fail_timeout;
        self
    }
}

#[derive(Debug)]
pub enum UpstreamError {}

#[async_trait]
pub trait Upstream: Send + Sync {
    async fn next(&mut self) -> Result<net::SocketAddr, UpstreamError>;
}

pub struct UpstreamImpl<S> {
    hosts: Vec<Host>,
    resolver: Arc<tokio::sync::Mutex<Resolver>>,
    sampler: S,
}

impl<S> UpstreamImpl<S> {
    pub fn new(hosts: Vec<Host>, resolver: Arc<tokio::sync::Mutex<Resolver>>, sampler: S) -> Self {
        Self {
            hosts,
            resolver,
            sampler,
        }
    }

    async fn resolve(&self, host: &str) -> Result<net::IpAddr, UpstreamError> {
        Ok(net::IpAddr::V4(net::Ipv4Addr::from_str("127.0.0.1").unwrap()))
    }
}

#[async_trait]
impl<S> Upstream for UpstreamImpl<S>
where
    S: Sampler + Send + Sync,
{
    async fn next(&mut self) -> Result<net::SocketAddr, UpstreamError> {
        if let Some(host) = self.hosts.get(self.sampler.sample()) {
            let ip = self.resolve(&host.hostname).await.unwrap();
            return Ok(net::SocketAddr::new(ip, host.port));
        } else {
            unreachable!("sample index out of range. this seems like a bug");
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::upstream::UpstreamImpl;
    use std::net;
    use std::sync::Arc;
    use std::time;

    use super::sampler::RoundRobinSampler;
    use super::Host;
    use super::Resolver;
    use super::Upstream;

    #[tokio::test]
    async fn test_upstream() {
        let hosts = vec![
            Host::builder("localhost", 8080)
                .with_weight(2)
                .with_max_conns(1024)
                .with_fail_timeout(time::Duration::from_secs(30))
                .build(),
            Host::builder("localhost", 8081)
                .with_weight(1)
                .with_max_conns(1024)
                .with_fail_timeout(time::Duration::from_secs(10))
                .build(),
        ];

        let mut us = UpstreamImpl::new(
            hosts.clone(),
            Arc::new(tokio::sync::Mutex::new(Resolver::new(None).unwrap())),
            RoundRobinSampler::new(hosts.len()).unwrap(),
        );

        assert_eq!(
            us.next().await.unwrap(),
            net::SocketAddrV4::new([127, 0, 0, 1].into(), 8080).into()
        );
        assert_eq!(
            us.next().await.unwrap(),
            net::SocketAddrV4::new([127, 0, 0, 1].into(), 8081).into()
        );
        assert_eq!(
            us.next().await.unwrap(),
            net::SocketAddrV4::new([127, 0, 0, 1].into(), 8080).into()
        );
        assert_eq!(
            us.next().await.unwrap(),
            net::SocketAddrV4::new([127, 0, 0, 1].into(), 8081).into()
        );
    }
}
