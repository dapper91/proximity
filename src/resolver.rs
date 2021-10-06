use std::collections;
use std::net;
use trust_dns_resolver::config::{NameServerConfigGroup, ResolverConfig, ResolverOpts};
pub use trust_dns_resolver::error::ResolveError;
use trust_dns_resolver::TokioAsyncResolver;

pub struct Resolver {
    resolver_impl: TokioAsyncResolver,
    // cache: collections::HashMap<String>,
}

impl Resolver {
    pub fn new(sockaddr: Option<net::SocketAddr>) -> Result<Self, ResolveError> {
        let resolver_impl = match sockaddr {
            None => TokioAsyncResolver::tokio_from_system_conf(),
            Some(sockaddr) => TokioAsyncResolver::tokio(
                ResolverConfig::from_parts(
                    None,
                    vec![],
                    NameServerConfigGroup::from_ips_clear(&[sockaddr.ip()], sockaddr.port(), true),
                ),
                ResolverOpts::default(),
            ),
        }?;

        Ok(Resolver { resolver_impl })
    }

    pub async fn resolve(&self, hostname: &str) -> Result<Vec<net::IpAddr>, ResolveError> {
        unimplemented!();
        // self.resolver_impl.lookup_ip(hostname);
    }
}
