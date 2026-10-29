# Charlotte server — native Rust

The `server/` directory is a standalone Cargo crate implemented in **Rust**, without Go source or Go build dependencies.

Run `cargo test --all-targets`, `cargo build --release --bins`, or `make build` from this folder.

Binaries: `charlotte-server` (HTTP + gRPC), `charlotte-ops` (signed deployment webhook and process supervisor), `charlotte-deploytoken` (deployment JWT signer), and `charlotte-chat` (interactive CLI). The original protobuf schema is compiled with bundled protoc.

```sh
cd server
export CHAT_JWT_SECRET="$(openssl rand -hex 32)"
cargo run --release --bin charlotte-server
curl http://127.0.0.1:8080/healthz
```

HTTP routes: POST /connect, POST /send, GET /poll, GET /events, GET /auth/me, GET /custom-metrics and GET /healthz; gRPC Chat port 50051. Port selection: HTTP_PORT, then PORT, default 8080.

Persisted state: CHAT_USERNAMES_FILE (data/usernames.log), CHAT_ROOMS_HLL_FILE (data/rooms.hll), CHAT_USERS_HLL_FILE (data/users.hll). HLL v2 data is compatible with the prior format. Supabase /auth/me needs SUPABASE_URL and SUPABASE_PUBLISHABLE_KEY. Optional CHARLOTTE_WEB_ORIGIN controls narrow CORS.

For the native deploy supervisor, set OPS_JWT_SECRET and run `cargo run --release --bin charlotte-ops`. Default OPS_ADDR is :9090; data directory OPS_DATA_DIR defaults to ops-data; managed health target OPS_APP_HEALTH_URL defaults to http://localhost:8080/healthz. OPS_DOWNLOAD_TOKEN supports private release assets.

Build the Docker image from this directory: `docker build -t charlotte-server:local .`.

**Deployment note:** Only deploy after Rust CI succeeds and feature parity is verified. If you previously used PUBLIC_HOSTNAME for automatic HTTPS certificates, terminate TLS at a reverse proxy; native automatic ACME certificate provisioning has not yet been ported.
