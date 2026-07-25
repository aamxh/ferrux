# Ferrux

Async reverse proxy built on [tokio](https://tokio.rs) with L7 HTTP routing, health checks, and weighted load balancing.

## Features

- **L4 / L7 proxying** — TCP passthrough or HTTP path-based routing
- **Weighted round-robin** — backends receive traffic proportional to their weight
- **Health checks** — background task pings backends every 5 seconds, skips dead ones
- **Buffer pool** — reused `BytesMut` buffers to minimize allocation overhead
- **Config-driven** — YAML config for backends, listen address, routing mode, and buffer settings

## Installation

```sh
cargo install ferrux
```

Or add to your `Cargo.toml`:

```toml
[dependencies]
ferrux = "0.1.0"
```

## Usage

1. Create a `config.yaml`:

```yaml
mode: l7  # or "l4" for TCP passthrough

listen:
  address: "127.0.0.1"
  port: 8080

backends:
  - address: "127.0.0.1"
    port: 9001
    path: "/api"
    weight: 3
  - address: "127.0.0.1"
    port: 9002
    path: "/"
    weight: 1

buffer_pool_size: 20
buffer_size: 8192
```

2. Run the server:

```sh
cargo run --bin server
```

3. Connect with any HTTP client:

```sh
curl http://127.0.0.1:8080/api/hello
```

## Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | `"l4"` \| `"l7"` | — | L4 = TCP passthrough, L7 = HTTP path routing |
| `listen.address` | `string` | — | Bind address |
| `listen.port` | `u16` | — | Bind port |
| `backends` | `array` | — | List of upstream servers |
| `backends[].address` | `string` | — | Backend address |
| `backends[].port` | `u16` | — | Backend port |
| `backends[].path` | `string` | `"/"` | URL prefix to route to this backend |
| `backends[].weight` | `usize` | `1` | Relative weight for load balancing |
| `buffer_pool_size` | `usize` | `20` | Number of pre-allocated buffers |
| `buffer_size` | `usize` | `8192` | Size of each buffer in bytes |

## Library Usage

```rust
use ferrux::{Config, load_backends, get_valid_backends, pick_backend, process};
```

All public types and functions are re-exported from the crate root.

## License

MIT

## Contributing

Contributions are welcome.
Feel free to fork this repository and submit a pull request.