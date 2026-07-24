use bytes::BytesMut;
use serde::Deserialize;
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
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

#[derive(Debug, Deserialize)]
struct Config {
    mode: String,
    backends: Vec<BackendServerConfig>,
    listen: ListenServerConfig,
    buffer_pool_size: Option<usize>,
    buffer_size: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ListenServerConfig {
    address: String,
    port: u16,
}

#[derive(Debug, Clone, Deserialize)]
struct BackendServerConfig {
    name: Option<String>,
    address: String,
    port: u16,
    path: Option<String>,
    #[serde(default = "default_weight")]
    weight: usize,
}

fn default_weight() -> usize {
    1
}

#[derive(Debug, Clone)]
struct Backend {
    server: BackendServerConfig,
    healthy: Arc<AtomicBool>,
}

type BackendPool = Arc<RwLock<Vec<Backend>>>;
type BufferPool = Arc<Mutex<Vec<BytesMut>>>;

#[tokio::main]
async fn main() {
    let config_content = tokio::fs::read_to_string("config.yaml").await.unwrap();
    let config: Config = serde_yaml::from_str(&config_content).unwrap();

    let listen_addr = get_addr_from_config(&config.listen.address, config.listen.port);
    let listener = TcpListener::bind(listen_addr).await.unwrap();
    println!("Server listening on {}", listen_addr);

    let backends_pool: BackendPool = Arc::new(RwLock::new(load_backends(&config)));
    let count = config.buffer_pool_size.unwrap_or(20);
    let buf_size = config.buffer_size.unwrap_or(8192);
    let counter = Arc::new(AtomicUsize::new(0));
    let buffers_pool: BufferPool = Arc::new(Mutex::new(
        (0..count)
            .map(|_| BytesMut::with_capacity(buf_size))
            .collect(),
    ));

    spawn_health_checker(backends_pool.clone());

    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();
        println!("Accepted connection from {}", addr);
        let counter = counter.clone();
        let backends_pool = backends_pool.clone();
        let buffers_pool = buffers_pool.clone();
        let mode = config.mode.clone();
        tokio::spawn(async move {
            let backends = get_valid_backends(&mode, backends_pool, &mut socket).await;
            if let Some(backends) = backends {
                let backend = pick_backend(backends, counter.clone());
                let addr = get_addr_from_config(&backend.server.address, backend.server.port);
                let (buf1, buf2) = {
                    let mut pool = buffers_pool.lock().unwrap();
                    (
                        pool.pop()
                            .unwrap_or_else(|| BytesMut::with_capacity(buf_size)),
                        pool.pop()
                            .unwrap_or_else(|| BytesMut::with_capacity(buf_size)),
                    )
                };
                let (processed_buf1, processed_buf2) =
                    process(socket, addr, buf1, buf2).await;
                {
                    let mut pool = buffers_pool.lock().unwrap();
                    pool.push(processed_buf1);
                    pool.push(processed_buf2);
                }
            } else {
                eprintln!("No healthy backends available for connection from {}", addr);
            }
        });
    }
}

fn get_addr_from_config(address: &str, port: u16) -> SocketAddr {
    format!("{}:{}", address, port)
        .parse()
        .unwrap()
}

fn load_backends(config: &Config) -> Vec<Backend> {
    config
        .backends
        .iter()
        .map(|backend| Backend {
            server: backend.clone(),
            healthy: Arc::new(AtomicBool::new(true)),
        })
        .collect()
}

fn spawn_health_checker(pool: BackendPool) {
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

async fn check_health(addr: SocketAddr) -> bool {
    match tokio::time::timeout(Duration::from_millis(500), TcpStream::connect(addr)).await {
        Ok(Ok(_)) => true,
        _ => false,
    }
}

async fn get_valid_backends(mode: &str, backends_pool: Arc<RwLock<Vec<Backend>>>, socket: &mut TcpStream) -> Option<Vec<Backend>> {
    let backends = backends_pool.read().await;
    let healthy_backends: Vec<_> = backends
        .iter()
        .cloned()
        .filter(|b| b.healthy.load(Ordering::Relaxed))
        .collect();

    if healthy_backends.is_empty() {
        return None;
    }

    if mode == "l4" {
        return Some(healthy_backends);
    }
    
    let mut buffer = Vec::new();
    let mut valid_backends: Vec<Backend> = Vec::new();
    loop {
        let n = socket.read_buf(&mut buffer).await.unwrap();

        if n == 0 {
            break;
        }

        let mut headers = [httparse::EMPTY_HEADER; 32];
        let mut req = httparse::Request::new(&mut headers);

        match req.parse(&buffer).unwrap() {
            httparse::Status::Complete(body_start) => {
                // Headers parsed

                // Read Content-Length if present
                let content_length = req
                    .headers
                    .iter()
                    .find(|h| h.name.eq_ignore_ascii_case("Content-Length"))
                    .and_then(|h| std::str::from_utf8(h.value).ok())
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0);

                // Wait until the whole body is available
                if buffer.len() < body_start + content_length {
                    continue;
                }

                // let body = &buffer[body_start..body_start + content_length];

                let path = req.path.unwrap_or("/");
                println!("Received request for path: {}", path);
                
                for backend in &healthy_backends {
                    if path.starts_with(&backend.server.path.as_deref().unwrap_or("/")) {
                        valid_backends.push(backend.clone());
                    }
                }

                break;
            }

            httparse::Status::Partial => continue,
        }
    }
    Some(valid_backends)
}

fn pick_backend(backends: Vec<Backend>, counter: Arc<AtomicUsize>) -> Backend {
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

async fn process(
    mut socket: TcpStream,
    addr: SocketAddr,
    mut buf1: BytesMut,
    mut buf2: BytesMut,
) -> (BytesMut, BytesMut) {
    println!("Forwarding connection to {}", addr);
    let mut upstream = match TcpStream::connect(addr).await {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("Failed to connect to upstream: {}", err);
            return (buf1, buf2);
        }
    };

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
                    Err(_) => break,
                }
            }
        }
    }
    return (buf1, buf2);
}
