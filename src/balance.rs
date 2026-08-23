use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use crate::config::Backend;

pub fn pick_backend(backends: Vec<Backend>, counter: Arc<AtomicUsize>) -> Backend {
    let total_weight: usize = backends.iter().map(|b| b.server.weight).sum();
    let index = counter.fetch_add(1, Ordering::Relaxed) % total_weight;

    let mut cumulative = 0;
    for backend in &backends {
        cumulative += backend.server.weight;
        if index < cumulative {
            return backend.clone();
        }
    }
    backends[0].clone()
}
