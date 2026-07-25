pub mod config;
pub mod error;

pub use config::{Backend, BackendServerConfig, Config, ListenServerConfig};
pub use error::HttpError;

use bytes::BytesMut;
use std::{
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::RwLock,
};

const MAX_HEADER_SIZE: usize = 8192;

pub type BackendPool = Arc<RwLock<Vec<Backend>>>;
pub type BufferPool = Arc<Mutex<Vec<BytesMut>>>;

pub fn get_addr_from_config(address: &str, port: u16) -> SocketAddr {
    format!("{}:{}", address, port)
        .parse()
        .unwrap()
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

pub async fn get_valid_backends(
    mode: &str,
    backends_pool: Arc<RwLock<Vec<Backend>>>,
    socket: &mut TcpStream,
) -> Result<(Vec<Backend>, Vec<u8>), HttpError> {
    let healthy_backends: Vec<_> = {
        let backends = backends_pool.read().await;
        backends
            .iter()
            .cloned()
            .filter(|b| b.healthy.load(Ordering::Relaxed))
            .collect()
    };
    if healthy_backends.is_empty() {
        return Err(HttpError::ServiceUnavailable);
    }

    if mode == "l4" {
        return Ok((healthy_backends, Vec::new()));
    }

    let mut buffer = Vec::new();
    let mut valid_backends: Vec<Backend> = Vec::new();
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if buffer.len() > MAX_HEADER_SIZE {
                eprintln!("Request headers too large!");
                return Err(HttpError::HeaderTooLarge);
            }

            let n = socket.read_buf(&mut buffer).await.ok().unwrap_or(0);
            if n == 0 {
                return Err(HttpError::BadRequest);
            }

            let mut headers = [httparse::EMPTY_HEADER; 32];
            let mut req = httparse::Request::new(&mut headers);

            match req.parse(&buffer).ok().unwrap_or(httparse::Status::Partial) {
                httparse::Status::Complete(body_start) => {
                    let content_length = req
                        .headers
                        .iter()
                        .find(|h| h.name.eq_ignore_ascii_case("Content-Length"))
                        .and_then(|h| std::str::from_utf8(h.value).ok())
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(0);

                    if buffer.len() < body_start + content_length {
                        continue;
                    }

                    let path = req.path.unwrap_or("/");
                    println!("Received request for path: {}", path);

                    for backend in &healthy_backends {
                        if path.starts_with(&backend.server.path.as_deref().unwrap_or("/")) {
                            valid_backends.push(backend.clone());
                        }
                    }

                    if valid_backends.is_empty() {
                        println!("No valid backends found for path: {}", path);
                        return Err(HttpError::NotFound);
                    }

                    return Ok(valid_backends);
                }
                httparse::Status::Partial => continue,
            }
        }
    })
    .await;

    match result {
        Ok(Ok(backends)) => Ok((backends, buffer)),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(HttpError::BadRequest),
    }
}

pub fn pick_backend(backends: Vec<Backend>, counter: Arc<AtomicUsize>) -> Backend {
    let total_weight: usize = backends.iter().map(|b| b.server.weight).sum();
    let index = counter.fetch_add(1, Ordering::Relaxed) % total_weight;

    let mut cumulative = 0;
    for backend in &backends {
        cumulative += backend.server.weight;
        if index < cumulative {
            return backend.clone();
        }
    }
    backends[0].clone()
}

pub async fn process(
    mut socket: TcpStream,
    addr: SocketAddr,
    initial_data: Vec<u8>,
    mut buf1: BytesMut,
    mut buf2: BytesMut,
) -> (BytesMut, BytesMut) {
    println!("Forwarding connection to {}", addr);
    let mut upstream = match TcpStream::connect(addr).await {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("Failed to connect to upstream: {}", err);
            socket.write_all(HttpError::Internal.response()).await.ok();
            return (buf1, buf2);
        }
    };

    if !initial_data.is_empty() {
        if upstream.write_all(&initial_data).await.is_err() {
            eprintln!("Failed to send initial data to upstream");
            socket.write_all(HttpError::Internal.response()).await.ok();
            return (buf1, buf2);
        }
    }

    loop {
        buf1.clear();
        buf2.clear();

        tokio::select! {
            result = socket.read_buf(&mut buf1) => {
                match result {
                    Ok(0) => break,
                    Ok(n) => {
                        if upstream.write_all(&buf1[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            result = upstream.read_buf(&mut buf2) => {
                match result {
                    Ok(0) => break,
                    Ok(n) => {
                        if socket.write_all(&buf2[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        socket.write_all(HttpError::Internal.response()).await.ok();
                        break;
                    }
                }
            }
        }
    }
    (buf1, buf2)
}
