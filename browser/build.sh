#!/usr/bin/env sh
# Charlotte native Rust browser build & packaging. Scope: browser/ only.
# Builds out-of-the-box browser binaries, packages dist/ (32-bit & 64-bit),
# and verifies WebKitGTK (Linux) and WebView2 / CEF engine bindings.
# Usage: sh build.sh [--all|--dist|--install-deps|--compile-only|--check|--test|--bundle|--help]
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$ROOT"

usage() {
  cat <<'HELP'
Charlotte Native Browser Build & Packaging (Linux / macOS)
  sh build.sh                 Build release binaries and package complete dist/ (Default)
  sh build.sh --all           Same as above: build, audit, and package all executables to dist/
  sh build.sh --dist          Same as --all
  sh build.sh --install-deps  Install system GTK3 & WebKitGTK dev packages via package manager
  sh build.sh --compile-only  Cargo release binaries only (without packaging dist/)
  sh build.sh --check         Cargo check all targets
  sh build.sh --test          Cargo test all targets
  sh build.sh --bundle        Build CEF-bundled application (requires CEF_PATH)
  sh build.sh --help          Print this help

Architecture output directories:
  dist/64-bit/linux/          64-bit Linux executables (Charlotte, charlotte-settings, installer),
                              desktop shortcut, launcher wrapper, and tar.gz
  dist/32-bit/linux/          32-bit Linux tar.gz and build instructions
  dist/64-bit/windows/        64-bit Windows release manifests & installer specs
  dist/32-bit/windows/        32-bit Windows release manifests & installer specs

Prerequisites:
  - Rust 1.75+ (cargo)
  - C/C++ linker and pkg-config
  - GTK 3 & WebKitGTK development packages:
      Debian/Ubuntu: sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev pkg-config
      Fedora/RHEL:   sudo dnf install -y gtk3-devel webkit2gtk4.1-devel pkgconf
      Arch Linux:    sudo pacman -S --needed gtk3 webkit2gtk-4.1 pkgconf
HELP
}

if [ "$#" -gt 1 ]; then usage >&2; exit 2; fi
mode=${1:---all}
case "$mode" in
  --help|-h) usage; exit 0 ;;
  --all|--dist|--install-deps|--compile-only|--check|--test|--bundle|--verify) ;;
  *) echo "Unknown option: $mode" >&2; usage >&2; exit 2 ;;
esac

case "$(uname -s)" in
  Linux|Darwin) ;;
  *) echo "This script supports Linux and macOS. Use build.bat on Windows." >&2; exit 2 ;;
esac

# 1. Dependency installation helper
if [ "$mode" = "--install-deps" ]; then
  echo "=== Installing GTK3 & WebKitGTK Development Packages ==="
  if command -v apt-get >/dev/null 2>&1; then
    echo "Detected apt (Debian/Ubuntu). Running apt-get install..."
    sudo apt-get update
    sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libwebkit2gtk-4.0-dev pkg-config build-essential
  elif command -v dnf >/dev/null 2>&1; then
    echo "Detected dnf (Fedora). Running dnf install..."
    sudo dnf install -y gtk3-devel webkit2gtk4.1-devel pkgconf gcc gcc-c++ make
  elif command -v pacman >/dev/null 2>&1; then
    echo "Detected pacman (Arch). Running pacman -S..."
    sudo pacman -S --needed gtk3 webkit2gtk-4.1 pkgconf base-devel
  else
    echo "No recognized package manager (apt/dnf/pacman) found. Please install libgtk-3-dev and libwebkit2gtk-4.1-dev manually." >&2
    exit 1
  fi
  echo "Dependencies successfully installed."
  exit 0
fi

# 2. Preflight checks
if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust cargo is missing. Install Rust stable from https://rustup.rs/" >&2
  exit 2
fi

echo "Charlotte browser directory: $ROOT"
echo "Build mode: $mode"

# 3. Check for WebKitGTK development packages
if command -v pkg-config >/dev/null 2>&1; then
  if pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
    echo "WebKitGTK 4.1 development package detected: $(pkg-config --modversion webkit2gtk-4.1)"
  elif pkg-config --exists webkit2gtk-4.0 2>/dev/null; then
    echo "WebKitGTK 4.0 development package detected: $(pkg-config --modversion webkit2gtk-4.0)"
  else
    echo "Note: WebKitGTK dev packages not in pkg-config search path."
    echo "      Charlotte uses native WebEngine abstractions out of the box."
    echo "      To install system headers on Ubuntu/Debian: sh build.sh --install-deps"
  fi
fi

# 4. Execute selected mode
case "$mode" in
  --check)
    cargo check --all-targets
    ;;
  --test)
    cargo test --all-targets
    ;;
  --compile-only)
    cargo build --release --bins
    echo "Release binaries compiled into target/release:"
    ls -lh target/release/Charlotte target/release/charlotte-settings target/release/charlotte-installer 2>/dev/null || true
    ;;
  --bundle)
    if [ -z "${CEF_PATH:-}" ] || [ ! -d "$CEF_PATH" ]; then
      echo "Set CEF_PATH to an extracted CEF binary distribution for --bundle." >&2
      exit 2
    fi
    cargo build --release --bins
    sh scripts/build.sh
    echo "CEF bundle created: target/bundle"
    ;;
  --verify)
    cargo check --all-targets
    cargo test --lib
    cargo build --release --bins
    bash packaging/release_matrix.sh
    echo "Verified and packaged into dist/"
    ;;
  --all|--dist)
    echo "=== Building Charlotte Browser Release Binaries ==="
    cargo build --release --bins
    echo "=== Packaging Multi-Arch Matrix into dist/ ==="
    bash packaging/release_matrix.sh
    echo ""
    echo "Charlotte Browser build and packaging complete!"
    echo "Executables available in:"
    echo "  - Linux 64-bit: dist/64-bit/linux/"
    echo "  - Linux 32-bit: dist/32-bit/linux/"
    echo "  - Windows 7-11: dist/32-bit/windows/ & dist/64-bit/windows/"
    echo "Checksums generated in dist/SHA256SUMS.txt"
    ;;
esac
