#!/usr/bin/env bash
# Cross builds with Rust targets and toolchains; host must provide linker.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p dist
targets=(linux/amd64)
if [[ -n "$TARGETS" ]]; then read -r -a targets <<< "$TARGETS"; fi
for spec in "$(printf '%s\n' "$@")"; do :; done
for spec in "${targets[@]}"; do
  case "$spec" in
    linux/amd64) target=x86_64-unknown-linux-gnu ;;
    linux/arm64) target=aarch64-unknown-linux-gnu ;;
    windows/amd64) target=x86_64-pc-windows-gnu ;;
    darwin/amd64) target=x86_64-apple-darwin ;;
    darwin/arm64) target=aarch64-apple-darwin ;;
    freebsd/amd64) target=x86_64-unknown-freebsd ;;
    *) echo "unsupported target: $spec" >&2; exit 2 ;;
  esac
  rustup target add "$target"
  cargo build --release --target "$target" --bins
  os="$(echo "$spec" | cut -d/ -f1)"
  arch="$(echo "$spec" | cut -d/ -f2)"
  suffix=""
  if [[ "$os" == windows ]]; then suffix=".exe"; fi
  for tool in server ops; do
    cp "target/$target/release/charlotte-$tool$suffix" "dist/chat-service-$tool-$os-$arch$suffix"
  done
done
(cd dist && (sha256sum chat-service-* || shasum -a 256 chat-service-*) > checksums.txt)
