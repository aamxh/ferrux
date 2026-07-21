use tokio::{
    net::{TcpStream},
    io::{AsyncReadExt, AsyncWriteExt},
};

#[tokio::main]
async fn main() {
    let mut stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();

    // sending bytes
    stream.write_all(b"Hello from client!").await.unwrap();

    // receiving bytes
    let mut buffer = [0u8; 1024];
    let n = stream.read(&mut buffer).await.unwrap();

    println!("Received {} bytes", n);

}