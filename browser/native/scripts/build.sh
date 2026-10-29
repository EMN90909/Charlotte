#!/usr/bin/env sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
[ "$#" -eq 0 ] || { echo "Build on the target operating system; cross-target packaging is not supported." >&2; exit 2; }
case "$(uname -s)" in
  Linux|Darwin) ;;
  *) echo "Unsupported host: $(uname -s)" >&2; exit 2 ;;
esac
cd "$ROOT"
# Both the application and the bundler must resolve the same persistent CEF
# distribution; the temporary build directory used by cargo install is removed.
: "${CEF_PATH:?Set CEF_PATH to a matching CEF distribution exported by export-cef-dir}"
[ -d "$CEF_PATH" ] || { echo "CEF_PATH is not a directory: $CEF_PATH" >&2; exit 2; }
export CEF_PATH
command -v bundle-cef-app >/dev/null 2>&1 || { echo "Install bundle-cef-app from cef 156.2.0+156.0.14 first." >&2; exit 2; }
case "$(uname -s)" in
  Darwin) bundle-cef-app Charlotte -o target/bundle --identifier com.noethric.charlotte --display-name Charlotte --version 0.4.0 ;;
  Linux) bundle-cef-app Charlotte -o target/bundle --release ;;
esac
