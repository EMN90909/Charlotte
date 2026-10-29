#!/usr/bin/env sh
# Charlotte native Rust + CEF browser build. Scope: browser/ only.
# Usage: sh build.sh [--bundle|--verify|--compile-only|--check|--test|--help]
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$ROOT"

usage() {
  cat <<'HELP'
Charlotte native browser build (Linux / macOS)
  sh build.sh                 Build release binaries and a runnable CEF bundle
  sh build.sh --bundle        Same as above
  sh build.sh --verify        Cargo check, tests, then release build + bundle
  sh build.sh --compile-only  Cargo release binaries only (NOT a runnable bundle)
  sh build.sh --check         Cargo check all targets
  sh build.sh --test          Cargo test all targets
  sh build.sh --help          Print this help

Prerequisites: Rust stable (cargo), native C/C++ linker, CEF distribution
matching Cargo.toml, and bundle-cef-app for a runnable bundle.
Set CEF_PATH to the extracted CEF binary distribution.
Everything is executed from browser/; no website/server build is invoked.
HELP
}

if [ "$#" -gt 1 ]; then usage >&2; exit 2; fi
mode=${1:---bundle}
case "$mode" in
  --help|-h) usage; exit 0 ;;
  --bundle|--verify|--compile-only|--check|--test) ;;
  *) echo "Unknown option: $mode" >&2; usage >&2; exit 2 ;;
esac

case "$(uname -s)" in
  Linux|Darwin) ;;
  *) echo "This script supports Linux and macOS. Use build.bat on Windows." >&2; exit 2 ;;
esac
if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust cargo is missing. Install Rust stable from https://rustup.rs/" >&2
  exit 2
fi
if [ "$mode" = "--bundle" ] || [ "$mode" = "--verify" ]; then
  if [ -z "${CEF_PATH:-}" ] || [ ! -d "$CEF_PATH" ]; then
    echo "Set CEF_PATH to the extracted, matching CEF binary distribution." >&2
    echo "A Rust-only executable is not a runnable CEF browser." >&2
    exit 2
  fi
  if ! command -v bundle-cef-app >/dev/null 2>&1; then
    echo "Missing bundle-cef-app from the CEF Rust build utilities." >&2
    echo "Install a version matching cef = 156.2.0+156.0.14." >&2
    exit 2
  fi
fi

echo "Charlotte browser directory: $ROOT"
echo "Build mode: $mode"
case "$mode" in
  --check)
    cargo check --all-targets
    ;;
  --test)
    cargo test --all-targets
    ;;
  --compile-only)
    cargo build --release --bins
    echo "Rust binaries: browser/target/release (not a runnable CEF bundle)"
    ;;
  --bundle)
    cargo build --release --bins
    sh scripts/build.sh
    echo "Runnable CEF bundle output: browser/target/bundle"
    ;;
  --verify)
    cargo check --all-targets
    cargo test --all-targets
    cargo build --release --bins
    sh scripts/build.sh
    echo "Verified build commands completed. Bundle: browser/target/bundle"
    ;;
esac
