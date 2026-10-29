FROM rust:1-bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --release --locked
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/* && useradd --uid 65532 --create-home app
WORKDIR /app
COPY --from=builder /build/target/release/charlotte-server /usr/local/bin/charlotte-server
USER 65532:65532
EXPOSE 8080 50051
CMD ["/usr/local/bin/charlotte-server"]
