# Charlotte Rust migration (in progress)

This is a **native Rust** implementation being developed alongside the original Go server.
Do not replace the Go deployment yet: feature parity has not been established.

## Implemented
- HTTP `/healthz`, `/connect`, `/send`, `/poll`, `/custom-metrics`
- HS256 JWT wire shape and MD5 URL room IDs compatible with existing Go clients
- Case-insensitive durable username claims, including reserved `system` name
- Bounded per-room message history, cursor polling, HTTP port selection, narrow CORS
- Rust unit tests for URL normalization and JWT round trip

## Not yet ported
- `/events` SSE streaming and its subscriber cap/reconnect semantics
- gRPC chat server and protobuf bindings on port 50051
- `/auth/me` Supabase access-token verification
- Exact Go HyperLogLog metrics and its file persistence (currently Rust uses exact in-memory counts)
- Graceful restart broadcast, TLS-ALPN autocert, and room garbage collection
- Ops manager, launcher, deploy token tool, release packaging, scripts, integration parity tests

The original `server/` Go files and deployment stay untouched until the above are
reimplemented, cross-tested and explicitly cut over.

## Build and test
```sh
cd server/rust
cargo fmt --check
cargo test
cargo build --release
CHAT_JWT_SECRET="$(openssl rand -hex 32)" ./target/release/charlotte-server
```

This branch's Rust code has not yet been compiled in CI. Avoid deploying it
as a drop-in production replacement.
