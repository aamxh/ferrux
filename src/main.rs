use std::{
    sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}},
    net::SocketAddr,
    time::Duration,
};
use tokio::{
    io::{copy_bidirectional}, 
    net::{TcpListener, TcpStream},
    sync::{RwLock},
};

#[derive(Debug, Clone)]
struct Backend {
    addr: SocketAddr,
    healthy: Arc<AtomicBool>, 
}

type BackendPool = Arc<RwLock<Vec<Backend>>>;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("Server listening on 127.0.0.1:8080");

    let backend_pool: BackendPool = Arc::new(RwLock::new(vec![
        Backend { addr: "127.0.0.1:9001".parse().unwrap(), healthy: Arc::new(AtomicBool::new(true)) },
        Backend { addr: "127.0.0.1:9002".parse().unwrap(), healthy: Arc::new(AtomicBool::new(true))   },
        Backend { addr: "127.0.0.1:9003".parse().unwrap(), healthy: Arc::new(AtomicBool::new(true)) },
    ]));
    let counter = Arc::new(AtomicUsize::new(0));

    spawn_health_checker(backend_pool.clone());

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        println!("Accepted connection from {}", addr);
        let counter = counter.clone();
        let pool = backend_pool.clone();
        tokio::spawn(async move {
            let backend = pickbackend(pool, counter).await;
            if let Some(backend) = backend {
                process(socket, backend.addr).await;
            } else {
                eprintln!("No healthy backends available for connection from {}", addr);
            }
        });
    }
}

fn spawn_health_checker(pool: BackendPool) {
    tokio::spawn(async move {
        loop {
            let backends = pool.read().await.clone(); // clone the Vec of Backends (cheap: just Arc clones inside)
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
        Ok(_) => true,
        Err(_) => false,
    }
}

async fn pickbackend(pool: Arc<RwLock<Vec<Backend>>>, counter: Arc<AtomicUsize>) -> Option<Backend> {
    let backends = pool.read().await;
    let healthy_backends: Vec<_> = backends.iter().filter(|b| b.healthy.load(Ordering::Relaxed)).collect();
    if healthy_backends.is_empty() {
        return None;
    }
    let counter_value = counter.fetch_add(1, Ordering::Relaxed) % healthy_backends.len();
    Some(healthy_backends[counter_value].clone())
}

async fn process(mut socket: TcpStream, addr: SocketAddr) {
    println!("Forwarding connection to {}", addr);
    let mut upstream = match TcpStream::connect(addr).await {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("Failed to connect to upstream: {}", err);
            return;
        }
    };
    if let Err(e) = copy_bidirectional(&mut socket, &mut upstream).await {
        eprintln!("Proxy error: {}", e);
    }
}