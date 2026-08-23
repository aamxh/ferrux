use bytes::BytesMut;
use std::sync::{Arc, Mutex};

pub type BufferPool = Arc<Mutex<Vec<BytesMut>>>;
