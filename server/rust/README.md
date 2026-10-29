# Charlotte native Rust replacement
This directory is the active migration workbench for the Go server; **do not deploy until CI succeeds and functional parity is verified**.

Run locally from `server/rust`:
```sh
cargo fmt --all
cargo test
cargo build --release
CHAT_JWT_SECRET="$(openssl rand -hex 32)" ./target/release/charlotte-server
```

HTTP listens on `HTTP_PORT` / `PORT` (default 8080); gRPC Chat on 50051. Protobuf definitions are shared with the original Go implementation under `server/proto/chat/chat.proto`.

Endpoints: POST /connect, POST /send, GET /poll, GET /events (SSE), GET /custom-metrics, GET /auth/me, GET /healthz. The chat protocol uses HS256 JWTs and MD5 URL-derived room IDs. Legacy Charlotte HLL v2 files and append-only username claims are read in place.

Docker from repo root:
```sh
docker build -f server/rust/Dockerfile -t charlotte-rust .
docker run --rm -p 8080:8080 -p 50051:50051 --env CHAT_JWT_SECRET -v charlotte-data:/app/data charlotte-rust
```

Still requires: Rust native ops manager and deploy token utilities, complete migration of standalone release/upgrade scripts, TLS/autocert, full behavioral parity tests for all Go integration cases, review of edge-case concurrency and restart semantics, and successful CI. The existing Go deployment remains unchanged.
