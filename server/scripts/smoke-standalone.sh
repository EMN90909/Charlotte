#!/usr/bin/env bash
# Runtime health smoke test against a freshly compiled standalone binary.
set -euo pipefail
cd "$(dirname "$0")/.."
make build
scratch="$(mktemp -d)"
pid=""
cleanup() {
  if [ -n "$pid" ]; then
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
  rm -rf "$scratch"
}
trap cleanup EXIT
port="${SMOKE_HTTP_PORT:-18180}"
env CHAT_JWT_SECRET="smoke-only-not-for-production" PORT="$port" \
  SUPABASE_URL="" SUPABASE_PUBLISHABLE_KEY="" \
  CHAT_USERNAMES_FILE="$scratch/usernames.log" \
  CHAT_ROOMS_HLL_FILE="$scratch/rooms.hll" \
  CHAT_USERS_HLL_FILE="$scratch/users.hll" \
  ./bin/charlotte-server >"$scratch/server.log" 2>&1 &
pid=$!
ready=0
for attempt in $(seq 1 40); do
  if ! kill -0 "$pid" 2>/dev/null; then
    cat "$scratch/server.log" >&2
    echo "Standalone server exited prematurely" >&2
    exit 1
  fi
  if curl --fail --silent --show-error --max-time 1 "http://127.0.0.1:$port/healthz" >"$scratch/health" 2>/dev/null; then
    ready=1
    break
  fi
  sleep 0.25
done
[ "$ready" -eq 1 ] || { cat "$scratch/server.log" >&2; echo "Health check timed out" >&2; exit 1; }
grep -qx 'ok' "$scratch/health"
# Without a configured Supabase project, identity validation must not be open.
status="$(curl --silent --max-time 3 --output /dev/null --write-out '%{http_code}' "http://127.0.0.1:$port/auth/me")"
[ "$status" = 503 ] || { echo "Expected /auth/me=503 when unconfigured, got $status" >&2; exit 1; }
echo "Standalone server smoke test passed (health 200, auth unconfigured 503)."
