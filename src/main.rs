use std::collections::HashMap;
use std::net;
use std::sync;

use clap::{arg_enum, value_t};
use env_logger;
use log;
use tokio;

mod config;
mod resolver;
mod server;
mod signal;
mod upstream;
mod utils;

use config::{Config, Proto};
use resolver::{ResolveError, Resolver};
use server::{Server, ServerError, TCPServer, UDPServer, UnixServer};
use signal::Signal;
use upstream::{Host, Upstream, UpstreamImpl};

arg_enum! {
#[derive(Debug)]
enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}
}

fn main() {
    let matches = clap::App::new("proximity")
        .arg(
            clap::Arg::with_name("config_path")
                .short("c")
                .long("config")
                .default_value("./config.yaml")
                .help("config file path"),
        )
        .arg(
            clap::Arg::with_name("log_level")
                .short("l")
                .long("loglevel")
                .default_value("info")
                .help("logging level")
                .possible_values(&LogLevel::variants())
                .case_insensitive(true),
        )
        .get_matches();

    let config_path = matches.value_of("config_path").unwrap();
    let log_level = value_t!(matches, "log_level", LogLevel).unwrap_or_else(|e| e.exit());

    env_logger::Builder::new()
        .filter_level(match log_level {
            LogLevel::Off => log::LevelFilter::Off,
            LogLevel::Error => log::LevelFilter::Error,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Trace => log::LevelFilter::Trace,
        })
        .format_timestamp_millis()
        .init();

    log::debug!("parsing configuration file ...");
    let config = match Config::parse_file(config_path) {
        Ok(config) => config,
        Err(e) => match *e.0 {
            config::ErrorImpl::Io(e) => {
                log::error!("configuration file reading error: {}", e);
                std::process::exit(1);
            }
            config::ErrorImpl::Parse(e) => {
                log::error!("configuration format error: {}", e);
                std::process::exit(1);
            }
        },
    };

    log::debug!("initializing services ...");
    let servers = match init(config) {
        Ok(servers) => servers,
        Err(e) => match e {
            InitializationError::ResolverError(e) => {
                log::error!("resolver initialization error: {}", e);
                std::process::exit(1);
            }
            InitializationError::ConfigError(e) => {
                log::error!("configuration error: {}", e);
                std::process::exit(1);
            }
        },
    };

    log::debug!("starting services ...");
    if serve(servers).is_err() {
        std::process::exit(1);
    }
}

enum InitializationError {
    ResolverError(ResolveError),
    ConfigError(String),
}

impl From<ResolveError> for InitializationError {
    fn from(error: ResolveError) -> Self {
        Self::ResolverError(error)
    }
}

fn init(config: Config) -> Result<Vec<Box<dyn Server>>, InitializationError> {
    let resolvers: Result<HashMap<String, Resolver>, ResolveError> = config
        .resolvers
        .iter()
        .map(|resolver_conf| {
            log::debug!("initializing resolver [{}] ...", resolver_conf.name);

            let sock_address = net::SocketAddr::new(resolver_conf.host, resolver_conf.port);

            return Ok((resolver_conf.name.clone(), Resolver::new(Some(sock_address))?));
        })
        .collect();

    let resolvers: HashMap<String, sync::Arc<tokio::sync::Mutex<Resolver>>> = resolvers?
        .into_iter()
        .map(|(name, resolver)| (name, sync::Arc::new(tokio::sync::Mutex::new(resolver))))
        .collect();

    let upstreams: Result<HashMap<String, Box<dyn Upstream>>, InitializationError> = config
        .upstreams
        .iter()
        .map(|upstream_conf| {
            log::debug!("initializing upstream [{}] ...", upstream_conf.name);

            let hosts = upstream_conf
                .hosts
                .iter()
                .map(|host_conf| {
                    Host::builder(&host_conf.host, host_conf.port)
                        .with_ipv6(host_conf.ipv6)
                        .with_fail_timeout(host_conf.fail_timeout)
                        .with_weight(host_conf.weight.unwrap_or(1))
                        .with_max_fails(host_conf.max_fails)
                        .with_max_conns(host_conf.max_conns)
                        .build()
                })
                .collect();

            let resolver = resolvers
                .get(&upstream_conf.resolver)
                .ok_or(InitializationError::ConfigError(format!(
                    "resolver [{}] not found",
                    upstream_conf.resolver
                )))?
                .clone();

            if upstream_conf.hosts.len() == 0 {
                return Err(InitializationError::ConfigError(format!(
                    "upstream {} host list is empty",
                    upstream_conf.name,
                )));
            }

            let upstream: Box<dyn Upstream> = match upstream_conf.hosts.iter().all(|host| host.weight.is_none()) {
                true => Box::new(UpstreamImpl::new(
                    hosts,
                    resolver,
                    upstream::sampler::RoundRobinSampler::new(upstream_conf.hosts.len()).map_err(|err| {
                        InitializationError::ConfigError(format!(
                            "upstream {} host weights are incorrect",
                            upstream_conf.name,
                        ))
                    })?,
                )),
                false => Box::new(UpstreamImpl::new(
                    hosts,
                    resolver,
                    upstream::sampler::WeightedSampler::new(
                        upstream_conf.hosts.iter().map(|host| host.weight.unwrap_or(1) as usize),
                    )
                    .map_err(|err| {
                        InitializationError::ConfigError(format!(
                            "upstream {} host weights are incorrect: {}",
                            upstream_conf.name, err,
                        ))
                    })?,
                )),
            };

            return Ok((upstream_conf.name.clone(), upstream));
        })
        .collect();

    let upstreams: HashMap<String, sync::Arc<tokio::sync::RwLock<Box<dyn Upstream>>>> = upstreams?
        .into_iter()
        .map(|(name, upstream)| (name, sync::Arc::new(tokio::sync::RwLock::new(upstream))))
        .collect();

    let servers: Result<Vec<Box<dyn Server>>, InitializationError> = config
        .servers
        .iter()
        .map(|server_conf| {
            log::info!("initializing server {}:{} ...", server_conf.host, server_conf.port);

            let upstream = upstreams
                .get(&server_conf.upstream)
                .ok_or(InitializationError::ConfigError(format!(
                    "upstream [{}] not found",
                    server_conf.upstream
                )))?
                .clone();

            let access_log_conf = server_conf.access_log.as_ref().unwrap_or(&config.access_log);
            let logger = env_logger::builder()
                // .format(
                //     |buf, record| {
                // strfmt::strfmt(&access_log_conf.format, )
                // writeln!(buf, access_log_conf.format, record.args())
                // }
                // )
                .format_timestamp(Some(match access_log_conf.timestamp_precision {
                    config::TimestampPrecision::Seconds => env_logger::TimestampPrecision::Seconds,
                    config::TimestampPrecision::Millis => env_logger::TimestampPrecision::Millis,
                    config::TimestampPrecision::Micros => env_logger::TimestampPrecision::Micros,
                    config::TimestampPrecision::Nanos => env_logger::TimestampPrecision::Nanos,
                }))
                .build();

            let logger: Box<dyn log::Log> = Box::new(logger);

            let result: Box<dyn Server> = match server_conf.proto {
                Proto::TCP => Box::new(TCPServer::new(server_conf.host, server_conf.port, upstream, logger)),
                Proto::UDP => Box::new(UDPServer::new(server_conf.host, server_conf.port, upstream, logger)),
                Proto::UNIX => Box::new(UnixServer::new(server_conf.host, server_conf.port, upstream, logger)),
            };

            return Ok(result);
        })
        .collect();

    return Ok(servers?);
}

enum RuntimeError {
    ServerError(Vec<ServerError>),
}

#[tokio::main]
async fn serve(servers: Vec<Box<dyn Server>>) -> Result<(), RuntimeError> {
    let (sig_sender, sig_receiver) = signal::signaler();

    let srv_handles: Vec<tokio::task::JoinHandle<_>> = servers
        .into_iter()
        .map(|server| {
            let sig_receiver = sig_receiver.clone();

            log::info!(
                "starting server {}://{}:{}",
                server.get_proto(),
                server.get_host(),
                server.get_port()
            );
            tokio::spawn(async move {
                let fut = server.start(sig_receiver);
                let result = fut.await;

                return result;
            })
        })
        .collect();

    let mut srv_watchdog = utils::wait_for_any(srv_handles);
    let srv_results = tokio::select! {
        res = tokio::signal::ctrl_c() => {
            res.unwrap();
            log::info!("received SIGINT. terminating ...");
            sig_sender.send(Signal::Stop);
            srv_watchdog.cease()
        },
        srv_results = &mut srv_watchdog => {
            log::info!("server task stopped. terminating ...");
            sig_sender.send(Signal::Stop);
            srv_results
        }
    };

    let mut errors = vec![];
    for future_result in utils::wait_for_all(srv_results).await {
        match future_result {
            utils::FutureState::Ready(Ok(srv_result)) => match srv_result {
                Ok(res) => {}
                Err(err) => {
                    log::error!("server stopped with an error: {}", err);
                    errors.push(err);
                }
            },
            utils::FutureState::Ready(Err(err)) => {
                if err.is_cancelled() {
                    unreachable!("server task is not supposed to be canceled. this seems like a bug")
                } else {
                    panic!("server task panicked")
                }
            }
            utils::FutureState::Pending(_) => {
                unreachable!("server task is not supposed to be pending at this point. this seems like a bug")
            }
        }
    }

    return if errors.is_empty() {
        Ok(())
    } else {
        Err(RuntimeError::ServerError(errors))
    };
}
