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

## Docker

Build the image:

```sh
docker build -t ferrux .
```

Run it, mounting your own config over the default baked into the image:

```sh
docker run -d --name ferrux \
  -p 8080:8080 \
  -v "$(pwd)/config.yaml:/etc/ferrux/config.yaml:ro" \
  ferrux
```

Notes:

- The container runs as an unprivileged user and reads its config from `/etc/ferrux/config.yaml`.
- Only the listener port needs publishing; connections to backends are outbound and require no extra ports.
- Backend addresses may be hostnames (e.g. `localhost`, Docker/k8s service names), resolved via the system resolver.

### Demo stack

A self-contained demo stands up the proxy plus three dummy HTTP backends on a private network:

```sh
docker compose up -d --build
```

Then try it:

```sh
curl localhost:8080/api/hello   # -> api-1 or api-2 (weighted round-robin, 1:2)
curl localhost:8080/static/app.js  # -> static-1
curl localhost:8080/nope        # -> 404 (no matching backend)
```

The backends are not published to the host. The proxy reaches them by service name (`api-1`, `api-2`, `static-1`) through `config.docker.yaml`, mirroring how the proxy would be wired up in a real deployment. Tear it down when done:

```sh
docker compose down
```

## Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | `"l4"` \| `"l7"` | — | L4 = TCP passthrough, L7 = HTTP path routing |
| `listen.address` | `string` | — | Bind address |
| `listen.port` | `u16` | — | Bind port |
| `backends` | `array` | — | List of upstream servers |
| `backends[].address` | `string` | — | Backend IP address or resolvable hostname |
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