FROM rust:1-bookworm AS builder
WORKDIR /build
COPY server/rust/ server/rust/
COPY server/proto/ server/proto/
WORKDIR /build/server/rust
RUN cargo build --release
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/* && useradd -u 65532 -m charlotte
WORKDIR /app
COPY --from=builder /build/server/rust/target/release/charlotte-server /usr/local/bin/charlotte-server
USER 65532
EXPOSE 8080 50051
CMD ["charlotte-server"]
