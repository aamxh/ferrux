use std::{
    sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}, Mutex},
    net::SocketAddr,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt}, 
    net::{TcpListener, TcpStream},
    sync::RwLock,
};
use serde::Deserialize;
use bytes::BytesMut;

#[derive(Debug, Deserialize)]
struct Config {
    backends: Vec<ServerConfig>,
    listen: ServerConfig,
    buffer_pool_size: Option<usize>,
    buffer_size: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ServerConfig {
    address: String,
    port: u16,
}

#[derive(Debug, Clone)]
struct Backend {
    addr: SocketAddr,
    healthy: Arc<AtomicBool>, 
}

type BackendPool = Arc<RwLock<Vec<Backend>>>;
type BufferPool = Arc<Mutex<Vec<BytesMut>>>;

#[tokio::main]
async fn main() {
    let config_content = tokio::fs::read_to_string("config.yaml").await.unwrap();
    let config: Config = serde_yaml::from_str(&config_content).unwrap();

    let listen_addr = load_listen_addr(&config);
    let listener = TcpListener::bind(listen_addr).await.unwrap();
    println!("Server listening on {}", listen_addr);

    let backends_pool: BackendPool = Arc::new(RwLock::new(load_backends(&config)));
    let counter = Arc::new(AtomicUsize::new(0));
    let count = config.buffer_pool_size.unwrap_or(20);
    let buf_size = config.buffer_size.unwrap_or(8192);
    let buffers_pool: BufferPool = Arc::new(Mutex::new((0..count).map(|_| BytesMut::with_capacity(buf_size)).collect()));

    spawn_health_checker(backends_pool.clone());

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        println!("Accepted connection from {}", addr);
        let counter = counter.clone();
        let backends_pool = backends_pool.clone();
        let buffers_pool = buffers_pool.clone();
        tokio::spawn(async move {
            let backend = pickbackend(backends_pool, counter).await;
            if let Some(backend) = backend {
                let (buf1, buf2) = {
                    let mut pool = buffers_pool.lock().unwrap();
                    (pool.pop().unwrap_or_else(|| BytesMut::with_capacity(buf_size)),
                    pool.pop().unwrap_or_else(|| BytesMut::with_capacity(buf_size)))
                };
                let (processed_buf1, processed_buf2) = process(socket, backend.addr, buf1, buf2).await;
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

fn load_listen_addr(config: &Config) -> SocketAddr {
    format!("{}:{}", config.listen.address, config.listen.port).parse().unwrap()
}

fn load_backends(config: &Config) -> Vec<Backend> {
    config.backends.iter().map(|server| {
        let addr = format!("{}:{}", server.address, server.port).parse().unwrap();
        Backend {
            addr,
            healthy: Arc::new(AtomicBool::new(true)), // Assume healthy initially
        }
    }).collect()
}

fn spawn_health_checker(pool: BackendPool) {
    tokio::spawn(async move {
        loop {
            let backends = pool.read().await.clone();
            for backend in backends {
                let healthy = check_health(backend.addr).await;
                backend.healthy.store(healthy, Ordering::Relaxed);
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

async fn check_health(addr: SocketAddr) -> bool {
    match tokio::time::timeout(
        Duration::from_millis(500),
        TcpStream::connect(addr)
    ).await {
        Ok(Ok(_)) => true,
        _ => false,
    }
}

async fn pickbackend(backends_pool: Arc<RwLock<Vec<Backend>>>, counter: Arc<AtomicUsize>) -> Option<Backend> {
    let backends = backends_pool.read().await;
    let healthy_backends: Vec<_> = backends.iter().filter(|b| b.healthy.load(Ordering::Relaxed)).collect();
    if healthy_backends.is_empty() {
        return None;
    }
    let counter_value = counter.fetch_add(1, Ordering::Relaxed) % healthy_backends.len();
    Some(healthy_backends[counter_value].clone())
}

async fn process(mut socket: TcpStream, addr: SocketAddr, mut buf1: BytesMut, mut buf2: BytesMut) -> (BytesMut, BytesMut) {
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