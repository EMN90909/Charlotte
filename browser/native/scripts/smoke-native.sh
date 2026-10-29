#!/usr/bin/env bash
# Launch bundled CEF in Xvfb and exercise Rust-hosted tab/sidebar UI.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "$(uname -s)" != Linux ]]; then
  echo "This smoke harness targets Linux" >&2; exit 2
fi
: "${CEF_PATH:?Set CEF_PATH to a matching CEF binary distribution}"
command -v xvfb-run >/dev/null || { echo 'Install xvfb' >&2; exit 2; }
command -v curl >/dev/null || { echo 'Install curl' >&2; exit 2; }
if [[ ! -d target/bundle ]]; then
  command -v bundle-cef-app >/dev/null || { echo 'Missing bundle-cef-app' >&2; exit 2; }
  bundle-cef-app Charlotte -o target/bundle --release
fi
APP="$(find target/bundle -type f -name Charlotte -perm /u+x -print -quit)"
[[ -n "$APP" ]] || { echo 'CEF bundle has no executable Charlotte browser' >&2; exit 1; }
PORT_FILE="$(mktemp)"
rm -f "$PORT_FILE"
LOG="$(mktemp)"
export CHARLOTTE_UI_PORT_FILE="$PORT_FILE"
export CHARLOTTE_PROFILE_DIR="$(mktemp -d)"
chmod 700 "$CHARLOTTE_PROFILE_DIR"
xvfb-run -a "$APP" >"$LOG" 2>&1 &
pid=$!
cleanup() {
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  rm -f "$PORT_FILE" "$LOG"
  rm -rf "$CHARLOTTE_PROFILE_DIR"
}
trap cleanup EXIT
port=""
for _ in $(seq 1 100); do
  if [[ -s "$PORT_FILE" ]]; then
    port="$(cat "$PORT_FILE")"
    if curl --silent --fail --max-time 1 "http://127.0.0.1:$port/chrome" > /dev/null; then break; fi
  fi
  if ! kill -0 "$pid" 2>/dev/null; then
    echo "Charlotte exited before CEF UI initialized" >&2
    cat "$LOG" >&2; exit 1
  fi
  sleep 0.3
done
[[ "$port" =~ ^[0-9]+$ ]] || { cat "$LOG" >&2; echo 'Native UI port not found' >&2; exit 1; }
curl --silent --fail "http://127.0.0.1:$port/chrome" | grep -q "Charlotte"
curl --silent --fail "http://127.0.0.1:$port/sidebar" | grep -q "Ask AI"
code="$(curl --silent --output /dev/null --write-out '%{http_code}' \
  -X POST "http://127.0.0.1:$port/action" -d 'command=new')"
[[ "$code" == 403 ]] || { echo "Expected cross-origin rejection, got $code" >&2; exit 1; }
kill -0 "$pid"
echo 'Native CEF process, toolbar/sidebar endpoints and CSRF rejection passed.'
