use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("Server listening on 127.0.0.1:8080");

    loop {
        let (mut socket, addr) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            println!("Accepted connection from {}", addr);

            let mut buf = [0u8; 1024];
            loop {

                // receiving bytes
                let n = socket.read(&mut buf).await.unwrap();
                if n == 0 {
                    break; // connection closed
                }
                println!("Received {} bytes from {}", n, addr);

                // sending bytes back 
                socket.write_all(&buf[..n]).await.unwrap();
            }
        });
    }
}