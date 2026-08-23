use std::{
    net::SocketAddr,
    sync::{
        Arc, atomic::Ordering,
    },
    time::Duration,
};
use tokio::{net::TcpStream, sync::RwLock};

use crate::config::{Backend, get_addr_from_config};

pub type BackendPool = Arc<RwLock<Vec<Backend>>>;

pub fn spawn_health_checker(pool: BackendPool) {
    tokio::spawn(async move {
        loop {
            let backends = pool.read().await.clone();
            for backend in backends {
                let addr = get_addr_from_config(&backend.server.address, backend.server.port);
                let healthy = check_health(addr).await;
                backend.healthy.store(healthy, Ordering::Relaxed);
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

pub async fn check_health(addr: SocketAddr) -> bool {
    match tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(addr)).await {
        Ok(Ok(_)) => true,
        _ => false,
    }
}
