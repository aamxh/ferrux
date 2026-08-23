#[derive(Debug)]
pub enum HttpError {
    BadRequest,
    NotFound,
    HeaderTooLarge,
    ServiceUnavailable,
    Internal,
}

impl HttpError {
    pub fn response(&self) -> &'static [u8] {
        match self {
            HttpError::BadRequest =>
                b"HTTP/1.1 400 Bad Request\r\nContent-Length:11\r\n\r\nBad Request",

            HttpError::NotFound =>
                b"HTTP/1.1 404 Not Found\r\nContent-Length:9\r\n\r\nNot Found",

            HttpError::HeaderTooLarge =>
                b"HTTP/1.1 413 Request Header Fields Too Large\r\nContent-Length:22\r\n\r\nRequest Header Fields Too Large",

            HttpError::ServiceUnavailable =>
                b"HTTP/1.1 503 Service Unavailable\r\nContent-Length:19\r\n\r\nService Unavailable",

            HttpError::Internal =>
                b"HTTP/1.1 500 Internal Server Error\r\nContent-Length:21\r\n\r\nInternal Server Error",
        }
    }
}
