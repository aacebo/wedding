# Build stage
FROM rust:slim AS builder

WORKDIR /app

COPY rust-toolchain.toml ./

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY bins ./bins

RUN cargo build --release --package api

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/api /app/api

# Static assets (hotel images) are served from disk at runtime, relative to the
# WORKDIR (/app), at bins/api/assets. SVGs are compiled into the binary via
# Askama include and don't need to be copied.
COPY --from=builder /app/bins/api/assets /app/bins/api/assets

CMD ["/app/api"]
