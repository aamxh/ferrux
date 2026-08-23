use bytes::BytesMut;
use std::sync::{Arc, atomic::AtomicUsize};
use tokio::{io::AsyncWriteExt, net::TcpListener};

use ferrux::{
    BackendPool, BufferPool, Config, get_addr_from_config, get_valid_backends, load_backends,
    pick_backend, process, spawn_health_checker,
};

#[tokio::main]
async fn main() {
    let config_content = tokio::fs::read_to_string("config.yaml").await.unwrap();
    let config: Config = serde_yaml::from_str(&config_content).unwrap();

    let listen_addr = get_addr_from_config(&config.listen.address, config.listen.port);
    let listener = TcpListener::bind(listen_addr).await.unwrap();
    println!("Server listening on {}", listen_addr);

    let backends_pool: BackendPool = Arc::new(tokio::sync::RwLock::new(load_backends(&config)));
    let count = config.buffer_pool_size.unwrap_or(20);
    let buf_size = config.buffer_size.unwrap_or(8192);
    let counter = Arc::new(AtomicUsize::new(0));
    let buffers_pool: BufferPool = Arc::new(std::sync::Mutex::new(
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
            let result = get_valid_backends(&mode, backends_pool, &mut socket).await;
            match result {
                Ok((backends, initial_data)) => {
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
                        process(socket, addr, initial_data, buf1, buf2).await;
                    {
                        let mut pool = buffers_pool.lock().unwrap();
                        pool.push(processed_buf1);
                        pool.push(processed_buf2);
                    }
                }
                Err(error) => {
                    eprintln!("Error processing request from {}: {:?}", addr, error);
                    let _ = socket.write_all(error.response()).await;
                }
            }
        });
    }
}
