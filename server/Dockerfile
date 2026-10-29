FROM rust:1-bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --release --bins

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 65532 --create-home charlotte
WORKDIR /app
COPY --from=builder /build/target/release/charlotte-server /usr/local/bin/charlotte-server
COPY --from=builder /build/target/release/charlotte-ops /usr/local/bin/charlotte-ops
ENV CHAT_USERNAMES_FILE=/app/data/usernames.log \
    CHAT_ROOMS_HLL_FILE=/app/data/rooms.hll \
    CHAT_USERS_HLL_FILE=/app/data/users.hll
RUN mkdir -p /app/data && chown 65532:65532 /app/data
EXPOSE 8080 50051
USER 65532:65532
CMD ["/usr/local/bin/charlotte-server"]
