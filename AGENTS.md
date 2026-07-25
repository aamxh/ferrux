# Ferrux

Async reverse proxy built on tokio. Published as a library crate with a binary server entry point.

## Build & Run

```sh
cargo build                        # build library + binary
cargo run --bin server              # starts server (reads config.yaml)
cargo run --example client          # connect to running server
cargo build --release               # optimized build
```

No tests, lints, or CI configured yet.

## Structure

```
src/
  lib.rs            — library crate: public types, proxy logic, health checks
  config.rs         — Config, BackendServerConfig, Backend structs
  error.rs          — HttpError enum with HTTP response mappings
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
