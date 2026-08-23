use serde::Deserialize;
use std::{
    net::SocketAddr,
    sync::{Arc, atomic::AtomicBool},
};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub mode: String,
    pub backends: Vec<BackendServerConfig>,
    pub listen: ListenServerConfig,
    pub buffer_pool_size: Option<usize>,
    pub buffer_size: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ListenServerConfig {
    pub address: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BackendServerConfig {
    pub address: String,
    pub port: u16,
    pub path: Option<String>,
    #[serde(default = "default_weight")]
    pub weight: usize,
}

fn default_weight() -> usize {
    1
}

#[derive(Debug, Clone)]
pub struct Backend {
    pub server: BackendServerConfig,
    pub healthy: Arc<AtomicBool>,
}

pub fn get_addr_from_config(address: &str, port: u16) -> SocketAddr {
    format!("{}:{}", address, port).parse().unwrap()
}

pub fn load_backends(config: &Config) -> Vec<Backend> {
    config
        .backends
        .iter()
        .map(|backend| Backend {
            server: backend.clone(),
            healthy: Arc::new(AtomicBool::new(true)),
        })
        .collect()
}
