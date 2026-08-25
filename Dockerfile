# syntax=docker/dockerfile:1

# ---- Cargo build stage ----
FROM rust:1.98.0-slim-bookworm AS builder

WORKDIR /usr/src/app

# Compile dependencies first with stub sources so they cache independently
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin \
    && echo "" > src/lib.rs \
    && echo "fn main() {}" > src/bin/server.rs \
    && cargo build --release --locked

# Copy real sources and rebuild only the ferrux crates
COPY src ./src
RUN touch src/lib.rs src/bin/server.rs \
    && cargo build --release --locked

# ---- Final runtime stage ----
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home ferrux

WORKDIR /etc/ferrux

COPY --from=builder /usr/src/app/target/release/server /usr/local/bin/ferrux
COPY config.yaml ./config.yaml

# Non-root user for security
USER ferrux

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/ferrux"]
