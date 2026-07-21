use tokio::{
    io::{copy_bidirectional}, net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("Server listening on 127.0.0.1:8080");

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        tokio::spawn(process(socket, addr));
    }
}

async fn process(mut socket: TcpStream, addr: core::net::SocketAddr) {
    println!("Accepted connection from {}", addr);
    let mut upstream = match TcpStream::connect("127.0.0.1:9000").await {
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