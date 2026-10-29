#!/usr/bin/env sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TARGET=${1:-}
case "$(uname -s)" in
  Linux|Darwin) ;;
  FreeBSD) [ -n "${CEF_PATH:-}" ] || { echo "FreeBSD is experimental: install system CEF and set CEF_PATH." >&2; exit 2; } ;;
  OpenBSD) echo "OpenBSD has no maintained CEF distribution; no browser binary can be built." >&2; exit 2 ;;
  *) echo "Unsupported host: $(uname -s)" >&2; exit 2 ;;
esac
cd "$ROOT"
if [ -n "$TARGET" ]; then cargo build --release --target "$TARGET"; else cargo build --release; fi
if ! command -v bundle-cef-app >/dev/null 2>&1; then
  cargo install cef --version 156.2.0+156.0.14 --bin bundle-cef-app --locked
fi
bundle-cef-app Charlotte -o target/bundle --release
