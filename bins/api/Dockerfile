# Build stage
FROM rust:alpine AS builder

WORKDIR /app

RUN apk add --no-cache musl-dev pkgconfig openssl-dev

COPY rust-toolchain.toml ./

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY bins ./bins

RUN cargo build --release --package api

# Runtime stage
FROM alpine:latest

WORKDIR /app

RUN apk add --no-cache ca-certificates libssl3

COPY --from=builder /app/target/release/api /app/api

# Static assets (hotel images) are served from disk at runtime, relative to the
# WORKDIR (/app), at bins/api/assets. SVGs are compiled into the binary via
# Askama include and don't need to be copied.
COPY --from=builder /app/bins/api/assets /app/bins/api/assets

CMD ["/app/api"]
