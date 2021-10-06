use std::fmt;
use std::fs;
use std::io;
use std::net;
use std::path::Path;
use std::str::FromStr;
use std::time;

type Result<T> = std::result::Result<T, ConfigError>;

pub struct ConfigError(pub Box<ErrorImpl>);

pub enum ErrorImpl {
    Io(io::Error),
    Parse(serde_yaml::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self.0 {
            ErrorImpl::Io(err) => write!(f, "configuration reading error: {}", err),
            ErrorImpl::Parse(err) => write!(f, "configuration parsing error: {}", err),
        }
    }
}

fn access_log_format() -> String {
    "{remote_addr}:{remote_port} to {upstream_addr}:{upstream_port} in {response_time} sec".into()
}

fn default_host_ipv6() -> bool {
    false
}

fn default_max_fails() -> u8 {
    1
}

fn default_max_conns() -> u16 {
    u16::MAX
}

fn default_max_queue_size() -> u16 {
    1024
}

fn default_max_queue_timeout() -> time::Duration {
    time::Duration::new(30, 0)
}

fn default_dns_host() -> net::IpAddr {
    [127, 0, 0, 1].into()
}

fn default_dns_port() -> u16 {
    53
}

fn default_dns_expiration() -> Option<time::Duration> {
    None
}

fn timestamp_precision() -> TimestampPrecision {
    TimestampPrecision::Millis
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Proto {
    TCP,
    UDP,
    UNIX,
}

impl fmt::Display for Proto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Proto::TCP => write!(f, "tcp"),
            Proto::UDP => write!(f, "udp"),
            Proto::UNIX => write!(f, "unix"),
        }
    }
}

#[derive(serde::Deserialize)]
pub struct Resolver {
    pub name: String,
    #[serde(default = "default_dns_host")]
    pub host: net::IpAddr,
    #[serde(default = "default_dns_port")]
    pub port: u16,

    #[serde(with = "humantime_serde", default = "default_dns_expiration")]
    pub expiration: Option<time::Duration>,
}

#[derive(serde::Deserialize, Default)]
pub struct Queue {
    #[serde(default = "default_max_queue_size")]
    pub size: u16,
    #[serde(with = "humantime_serde", default = "default_max_queue_timeout")]
    pub timeout: time::Duration,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimestampPrecision {
    Seconds,
    Millis,
    Micros,
    Nanos,
}

#[derive(serde::Deserialize)]
pub struct AccessLog {
    #[serde(default = "access_log_format")]
    pub format: String,
    #[serde(default = "timestamp_precision")]
    pub timestamp_precision: TimestampPrecision,
    pub file: Option<Box<Path>>,
}

#[derive(serde::Deserialize)]
pub struct Server {
    pub host: net::IpAddr,
    pub port: u16,
    pub proto: Proto,
    pub upstream: String,
    #[serde(default)]
    pub queue: Queue,
    pub access_log: Option<AccessLog>,
}

// pub struct HealthCheck{
//     #[serde(with = "humantime_serde")]
//     interval: time::Duration,
//     jitter: time::Duration,
//     fails: u8,
//     passes: i8,
//     port: Option<u16>,
// }

#[derive(serde::Deserialize)]
pub struct Host {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_host_ipv6")]
    pub ipv6: bool,
    pub weight: Option<u8>,
    #[serde(default = "default_max_fails")]
    pub max_fails: u8,
    #[serde(default = "default_max_conns")]
    pub max_conns: u16,
    #[serde(with = "humantime_serde")]
    pub fail_timeout: time::Duration,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StickyKind {
    IP,
}

#[derive(serde::Deserialize)]
pub struct Sticky {
    pub kind: StickyKind,
}

#[derive(serde::Deserialize)]
pub struct Upstream {
    pub name: String,
    pub hosts: Vec<Host>,
    pub resolver: String,
}

#[derive(serde::Deserialize)]
pub struct Config {
    pub access_log: AccessLog,
    pub servers: Vec<Server>,
    pub upstreams: Vec<Upstream>,
    pub resolvers: Vec<Resolver>,
}

impl Config {
    pub fn parse_file(path: &str) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(config) => Self::parse(&config),
            Err(e) => Err(ConfigError(Box::new(ErrorImpl::Io(e)))),
        }
    }

    pub fn parse(config: &str) -> Result<Self> {
        match serde_yaml::from_str(config) {
            Ok(config) => Ok(config),
            Err(e) => Err(ConfigError(Box::new(ErrorImpl::Parse(e)))),
        }
    }
}
