use std::{
    sync::{
        Arc, atomic::Ordering,
    },
    time::Duration,
};
use tokio::{
    io::AsyncReadExt,
    net::TcpStream,
    sync::RwLock,
};

use crate::config::Backend;
use crate::error::HttpError;

const MAX_HEADER_SIZE: usize = 8192;

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
