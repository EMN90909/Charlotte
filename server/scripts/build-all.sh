#!/usr/bin/env bash
# Cross-compile standalone Go server and ops-daemon binaries. Run in any
# working directory; only the server/ module is required.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p dist
# Keep original artifact names for existing release/deployment consumers.
targets=(linux/amd64 linux/arm64 darwin/amd64 darwin/arm64 windows/amd64)
if [ -n "${TARGETS:-}" ]; then
  read -r -a targets <<< "$TARGETS"
fi
for target in "${targets[@]}"; do
  case "$target" in
    linux/amd64|linux/arm64|darwin/amd64|darwin/arm64|windows/amd64|windows/arm64|freebsd/amd64|freebsd/arm64) ;;
    *) echo "Unsupported target $target" >&2; exit 2 ;;
  esac
  os="${target%/*}"
  arch="${target#*/}"
  ext=''
  if [ "$os" = windows ]; then ext='.exe'; fi
  for cmd in server ops; do
    output="dist/chat-service-${cmd}-${os}-${arch}${ext}"
    echo "Building $output"
    GOOS="$os" GOARCH="$arch" CGO_ENABLED=0 go build -trimpath -ldflags="-s -w" -o "$output" "./cmd/$cmd"
  done
done
(
  cd dist
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum chat-service-* > checksums.txt
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 chat-service-* > checksums.txt
  else
    echo "Install sha256sum or shasum to generate checksums" >&2
    exit 1
  fi
)
echo "Release artifacts and checksums in dist/"
