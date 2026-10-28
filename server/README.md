# Charlotte server — standalone Go service

The `server/` folder is a **self-contained Go module**. Build it without the
browser, website, parent repository's Makefile or scripts, or Node.js tooling.
The existing chat protocol and independently managed username claims remain
backwards-compatible.

## Requirements

- Go **1.23+** for native builds
- GNU Make and Bash for the optional convenience tasks; **not** required for
  direct `go build`
- Docker for container builds (optional)
- A strong, randomly generated `CHAT_JWT_SECRET` at runtime

## Build from this folder alone

From the Charlotte monorepo:

```sh
cd server
go mod download
make build                  # bin/charlotte-server
```

With **only this folder copied** to a separate location:

```sh
cp -R server ../charlotte-server
cd ../charlotte-server
go mod download
go build -trimpath -o charlotte-server ./cmd/server
```

The build requires no files outside the copied folder. On Windows, use
`go build -o charlotte-server.exe ./cmd/server` in PowerShell. No protobuf
compiler is needed for ordinary builds: generated Go bindings are committed in
`gen/chat/`.

### Run

```sh
# Generate your own secret first, e.g. openssl rand -hex 32
export CHAT_JWT_SECRET="REPLACE_WITH_A_LONG_RANDOM_SECRET"
./bin/charlotte-server
curl http://localhost:8080/healthz # returns "ok"
```

By default HTTP listens on **8080** and gRPC on **50051**. Set `PORT` (for
platform-managed hosts) or `HTTP_PORT` to override the HTTP port. If both are
set, `HTTP_PORT` wins. The program exits with an error for invalid ports.
`PUBLIC_HOSTNAME` is optional: enable native HTTPS only if the server owns the
hostname and can receive TLS-ALPN-01 traffic on port 443. Otherwise terminate
TLS at your ingress/reverse proxy and forward to HTTP.

The server creates `data/` relative to its current working directory for
durable username claims and metrics. Back up this directory and keep it across
restarts. The following environment variables relocate those files:

```text
CHAT_USERNAMES_FILE
CHAT_ROOMS_HLL_FILE
CHAT_USERS_HLL_FILE
AUTOCERT_CACHE_DIR
```

## Verify a real independent build

```sh
make check                  # go vet and go build of the module
make test                   # unit + integration tests (race detector)
make verify-standalone      # copies server/ to an isolated temp directory,
                            # then compiles/tests using only that copy
make smoke                  # builds, launches, and checks /healthz
```

`make smoke` defaults to port 18180 (override with `SMOKE_HTTP_PORT`). It
launches with a temporary test-only JWT signing secret and wipes temporary data.
Do not use that test secret in production.

## Docker (only server/ as build context)

```sh
cd server
docker build -t charlotte-server:local .
# In the same shell, set CHAT_JWT_SECRET to a secure value:
docker run --rm -p 8080:8080 -p 50051:50051 \
  --env CHAT_JWT_SECRET \
  -v charlotte-server-data:/app/data \
  charlotte-server:local
curl http://127.0.0.1:8080/healthz
```

The final image contains a statically compiled Go binary, runs as a **non-root**
user, and writes state under `/app/data`. A named Docker volume preserves the
claims log and metrics. For a host bind mount, make the directory writable by
UID/GID 65532. The secret must be supplied by the host, never baked into
Dockerfile layers or committed to Git.

## Independent release builds

```sh
make build-all
# or selectively:
TARGETS="linux/amd64 linux/arm64 windows/amd64" bash scripts/build-all.sh
```

Artifacts and SHA-256 checksums are placed in `dist/`. Historical
`chat-service-server-<os>-<arch>` and `chat-service-ops-<os>-<arch>` filenames
and `chat-service-v*` Git release tags are preserved for the existing ops
deployment automation. Local native builds are named `charlotte-server`.

If this folder becomes the root of its own GitHub repository, its bundled
`.github/workflows/build.yml` will compile/test it and upload release-format
binaries on each push; that nested workflow is intentionally not used by the
Charlotte monorepo, which has its own root workflows.

Version bumping, changelog stamping and tagging also work in an independent
Git checkout: `make bump-patch`, commit the change, then `make tag`. All
required scripts are included under `server/scripts/`.

## Supabase browser account verification

Supabase is **optional for server startup**. Configure the same Supabase project
as Charlotte browser using its public **publishable** key:

```sh
SUPABASE_URL=https://YOUR_PROJECT.supabase.co
SUPABASE_PUBLISHABLE_KEY=sb_publishable_YOUR_KEY
CHARLOTTE_WEB_ORIGIN=https://charlotte-9wyy.onrender.com
```

`GET /auth/me` accepts a Supabase user access token as
`Authorization: Bearer <token>` and verifies it with Supabase Auth before
returning the user's ID and email. It returns 401 for invalid tokens and 503
when Supabase is not configured. Do **not** supply Supabase secret/service-role
keys. `CHARLOTTE_WEB_ORIGIN` adds narrow CORS for public health/metrics
endpoints only.

**Important:** Browser sign-in and the legacy chat JWT/username protocol are
independent; signing into Charlotte does not yet require all chat participants
to authenticate with Supabase. No live Charlotte-specific Supabase project was
selected in this change.

See [PROTOCOL.md](PROTOCOL.md) for chat API usage, [deploy/README.md](deploy/README.md)
for the existing ops deployment workflow, and [LICENSE](LICENSE) for the
original Samuel Abolo MIT licence.
