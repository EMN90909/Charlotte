#!/usr/bin/env bash
# macOS CEF bundle process/localhost-UI acceptance test.
set -euo pipefail
[[ "$(uname -s)" == Darwin ]] || { echo 'macOS required' >&2; exit 2; }
cd "$(dirname "$0")/.."
APP="$(find target/bundle -type f -path '*/Contents/MacOS/Charlotte' -print -quit)"
[[ -x "$APP" ]] || { echo 'CEF .app/Contents/MacOS/Charlotte not found' >&2; exit 1; }
PORT_FILE="$(mktemp)"
rm -f "$PORT_FILE"
export CHARLOTTE_UI_PORT_FILE="$PORT_FILE"
export CHARLOTTE_PROFILE_DIR="$(mktemp -d)"
chmod 700 "$CHARLOTTE_PROFILE_DIR"
LOG="$(mktemp)"
"$APP" >"$LOG" 2>&1 &
pid=$!
cleanup(){
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
  if ! kill -0 "$pid" 2>/dev/null; then cat "$LOG" >&2; exit 1; fi
  sleep 0.3
done
[[ "$port" =~ ^[0-9]+$ ]] || { cat "$LOG" >&2; exit 1; }
curl --silent --fail "http://127.0.0.1:$port/chrome" | grep -q 'Charlotte'
curl --silent --fail "http://127.0.0.1:$port/chrome" | grep -q 'Toggle ad blocker'
curl --silent --fail "http://127.0.0.1:$port/sidebar" | grep -q 'Ask Charlotte'
curl --silent --fail "http://127.0.0.1:$port/sidebar" | grep -q 'Automate this Chromium tab'
curl --silent --fail "http://127.0.0.1:$port/sidebar" | grep -q 'Visual automation (shares a screenshot)'
curl --silent --fail "http://127.0.0.1:$port/sidebar" | grep -q 'Accept open dialog'
curl --silent --fail "http://127.0.0.1:$port/sidebar" | grep -q 'Browser skills'
curl --silent --fail "http://127.0.0.1:$port/sidebar" | grep -q 'MCP integrations directory'
mcp_code="$(curl --silent --output /dev/null --write-out '%{http_code}' -X POST "http://127.0.0.1:$port/mcp/catalog" -d 'catalog=github&command=enable')"
[[ "$mcp_code" == 403 ]] || { echo "Unsafe unsigned MCP catalog endpoint: $mcp_code" >&2; exit 1; }
skill_code="$(curl --silent --output /dev/null --write-out '%{http_code}' -X POST "http://127.0.0.1:$port/browser/skill" -d 'skill=summary')"
[[ "$skill_code" == 403 ]] || { echo "Unsafe unsigned browser skill endpoint: $skill_code" >&2; exit 1; }
automation_code="$(curl --silent --output /dev/null --write-out '%{http_code}'   -X POST "http://127.0.0.1:$port/browser/approve" -d 'command=approve')"
[[ "$automation_code" == 403 ]] || { echo "Unsafe browser automation approval endpoint: $automation_code" >&2; exit 1; }
for protected_path in /browser/control /browser/resume; do
  unsigned="$(curl --silent --output /dev/null --write-out '%{http_code}' -X POST "http://127.0.0.1:$port$protected_path" -d 'command=pause')"
  [[ "$unsigned" == 403 ]] || { echo "Unsafe automation checkpoint/control endpoint: $protected_path returned $unsigned" >&2; exit 1; }
done
for _ in $(seq 1 100); do
  if curl --silent --fail --max-time 2 "http://127.0.0.1:$port/health" | grep -Eq '"cef_main_frame_loads":[1-9][0-9]*'; then break; fi
  sleep 0.3
done
curl --silent --fail --max-time 2 "http://127.0.0.1:$port/health" | grep -Eq '"cef_main_frame_loads":[1-9][0-9]*' || {
  echo "CEF never completed a main-frame load" >&2; cat "$LOG" >&2; exit 1;
}

echo 'Native macOS CEF page render and Rust toolbar/sidebar smoke passed.'
