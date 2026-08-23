use bytes::BytesMut;
use std::net::SocketAddr;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::error::HttpError;

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

    if !initial_data.is_empty() && upstream.write_all(&initial_data).await.is_err() {
        eprintln!("Failed to send initial data to upstream");
        socket.write_all(HttpError::Internal.response()).await.ok();
        return (buf1, buf2);
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
