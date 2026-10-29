#!/usr/bin/env sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
[ "$#" -eq 0 ] || { echo "Build on the target operating system; cross-target packaging is not supported." >&2; exit 2; }
case "$(uname -s)" in
  Linux|Darwin) ;;
  *) echo "Unsupported host: $(uname -s)" >&2; exit 2 ;;
esac
cd "$ROOT"
cargo build --release
if ! command -v bundle-cef-app >/dev/null 2>&1; then
  cargo install cef --version 156.2.0+156.0.14 --bin bundle-cef-app --locked
fi
bundle-cef-app Charlotte -o target/bundle --release
