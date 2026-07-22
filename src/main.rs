use std::{
    sync::{Arc, Mutex},
    net::SocketAddr,
};
use tokio::{
    io::{copy_bidirectional}, 
    net::{TcpListener, TcpStream},
    sync::{RwLock},
};

#[derive(Debug, Clone)]
struct Backend {
    addr: SocketAddr,
    // health: bool, 
}

type BackendPool = Arc<RwLock<Vec<Backend>>>;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("Server listening on 127.0.0.1:8080");

    let backend_pool: BackendPool = Arc::new(RwLock::new(vec![
        Backend { addr: "127.0.0.1:9001".parse().unwrap() },
        Backend { addr: "127.0.0.1:9002".parse().unwrap() },
        Backend { addr: "127.0.0.1:9003".parse().unwrap() },
    ]));
    let counter = Arc::new(Mutex::new(0));

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        let counter = counter.clone();
        let pool = backend_pool.clone();
        tokio::spawn(async move {
            let backend = pickbackend(pool, counter).await;
            process(socket, backend.addr).await;
        });
    }
}

async fn pickbackend(pool: Arc<RwLock<Vec<Backend>>>, counter: Arc<Mutex<usize>>) -> Backend {
    let backends = pool.read().await;
    let mut counter_value = counter.lock().unwrap();
    *counter_value = (*counter_value + 1) % backends.len();
    backends[*counter_value].clone()
}

async fn process(mut socket: TcpStream, addr: SocketAddr) {
    println!("Accepted connection from {}", addr);
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