# ferrux

Async reverse proxy built on tokio. Learning project following a milestone-based approach.

## Build & Run

```sh
cargo build
cargo run                    # starts server on 127.0.0.1:8080
cargo run --example client   # connect to running server
```

No tests, lints, or CI configured yet.

## Structure

- `src/main.rs` — entry point
- `examples/client.rs` — example client

## Progress

- [x] Milestone 0: TCP echo server
- [ ] Milestone 1: Dumb TCP proxy (tokio::io::copy_bidirectional)
- [ ] Milestone 2: Multiple backends + round robin
- [ ] Milestone 3: Health checks
- [ ] Milestone 4: Config file support
- [ ] Milestone 5: Buffer pool
- [ ] Milestone 6: L7 HTTP routing

## Notes

- Rust edition 2024.
- Dependencies: `tokio` (full features), `bytes`.
