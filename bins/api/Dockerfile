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

CMD ["/app/api"]
