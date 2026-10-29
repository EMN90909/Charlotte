#!/usr/bin/env bash
# Measure the real packaged CEF browser, including its helper processes.
# This is observational; it deliberately does not impose arbitrary memory caps.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ "$(uname -s)" == Linux ]] || { echo "Linux required" >&2; exit 2; }
command -v xvfb-run >/dev/null || { echo "xvfb-run is required" >&2; exit 2; }
command -v curl >/dev/null || { echo "curl is required" >&2; exit 2; }
command -v ps >/dev/null || { echo "ps is required" >&2; exit 2; }
APP="$(find target/bundle -type f -name Charlotte -perm /u+x -print -quit)"
[[ -n "$APP" ]] || { echo "Build and package Charlotte with CEF first" >&2; exit 2; }
PORT_FILE="$(mktemp)"
LOG_FILE="$(mktemp)"
PROFILE="$(mktemp -d)"
rm -f "$PORT_FILE"
chmod 700 "$PROFILE"
export CHARLOTTE_UI_PORT_FILE="$PORT_FILE"
export CHARLOTTE_PROFILE_DIR="$PROFILE"
# Use a local new-tab page to avoid remote network variability during cold start.
unset CHARLOTTE_START_URL
started="$(date +%s%3N)"
xvfb-run -a "$APP" >"$LOG_FILE" 2>&1 &
pid=$!
cleanup(){
  kill "$pid" 2>/dev/null || true
  # The wrapper may have browser descendants; stop them before removing profile.
  for child in $(ps -eo pid=,ppid= | awk -v root="$pid" '$2==root{print $1}'); do
    kill "$child" 2>/dev/null || true
  done
  wait "$pid" 2>/dev/null || true
  rm -f "$PORT_FILE" "$LOG_FILE"
  rm -rf "$PROFILE"
}
trap cleanup EXIT
port=""
ready_flag=0
for _ in $(seq 1 100); do
  if [[ -s "$PORT_FILE" ]]; then
    port="$(cat "$PORT_FILE")"
    if [[ "$port" =~ ^[0-9]+$ ]] && curl --fail --silent --max-time 1 "http://127.0.0.1:$port/chrome" > /dev/null; then
      ready_flag=1
      break
    fi
  fi
  if ! kill -0 "$pid" 2>/dev/null; then
    cat "$LOG_FILE" >&2
    exit 1
  fi
  sleep 0.1
done
[[ "$ready_flag" == 1 ]] || { cat "$LOG_FILE" >&2; echo "CEF chrome UI never became ready" >&2; exit 1; }
ready="$(date +%s%3N)"
# Traverse descendants of xvfb-run (including Chromium renderer and GPU helpers).
# Xvfb itself is counted as part of the test harness, so the figure is an upper
# bound for the browser process tree, not a private-bytes measurement.
read -r processes rss_kib < <(ps -eo pid=,ppid=,rss= | awk -v root="$pid" '
  { parent[$1]=$2; rss[$1]=$3 }
  END {
    tree[root]=1
    for (round=0; round<32; round++) for (p in parent)
      if (tree[parent[p]]) tree[p]=1
    count=0; memory=0
    for (p in tree) if (p in rss) { count++; memory+=rss[p] }
    print count, memory
  }')
printf 'Charlotte cold-start UI ready: %s ms\n' "$((ready-started))"
printf 'CEF/Xvfb process-tree RSS sum (shared mappings double-counted): %s KiB across %s processes\n' "$rss_kib" "$processes"
printf 'Use this with equivalent hardware and profile sizes for before/after comparisons.\n'
