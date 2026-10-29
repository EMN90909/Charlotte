#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/isolated"
tar -c --exclude=target --exclude=dist --exclude=bin --exclude=.git -C . . | tar -x -C "$tmp/isolated"
cd "$tmp/isolated"
test -f Cargo.toml
test -f proto/chat/chat.proto
cargo test --all-targets
cargo build --release --bin charlotte-server
test -f target/release/charlotte-server
echo "Standalone Rust server build passed."
