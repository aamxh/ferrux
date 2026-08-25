# Ferrux

Async reverse proxy built on tokio. Published as a library crate with a binary server entry point.

## Build & Run

```sh
cargo build                        # build library + binary
cargo run --bin server              # starts server (reads config.yaml)
cargo run --example client          # connect to running server
cargo build --release               # optimized build
docker compose up -d --build        # full demo stack: proxy + 3 dummy backends
```

No tests, lints, or CI configured yet.

## Structure

```
src/
  lib.rs            — library crate root: module declarations + public API re-exports
  config.rs         — Config, BackendServerConfig, Backend structs, config helpers
  error.rs          — HttpError enum with HTTP response mappings
  router.rs         — L7 request parsing and path-based backend matching
  balance.rs        — backend selection (weighted round-robin)
  health.rs         — BackendPool type, background health checks
  proxy.rs          — bidirectional TCP forwarding loop
  buffer.rs         — BufferPool type alias
  bin/
    server.rs       — binary entry point (reads config, runs accept loop)
examples/
  client.rs         — example TCP client
```

## Publishing

```sh
cargo login <token>
cargo publish --dry-run    # verify before publishing
cargo publish
```

Update `repository` in `Cargo.toml` before publishing.

## Notes

- Rust edition 2024.
- Dependencies: `tokio` (full features), `bytes`, `serde`, `serde_yaml`, `httparse`.
