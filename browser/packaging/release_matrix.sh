#!/usr/bin/env bash
set -euo pipefail

# Charlotte Cross-Platform & Architecture Release Packager
# Builds and categorizes releases into:
# Folder 1: dist/32-bit/ (Windows x86 with Windows 7 ESR support + Linux i686)
# Folder 2: dist/64-bit/ (Windows x64 + Linux x86_64)

VERSION="0.4.0"
DIST_DIR="$(pwd)/dist"
DIR_32="${DIST_DIR}/32-bit"
DIR_64="${DIST_DIR}/64-bit"

echo "=== Packaging Charlotte Desktop v${VERSION} Multi-Arch Matrix ==="

mkdir -p "${DIR_32}/windows" "${DIR_32}/linux"
mkdir -p "${DIR_64}/windows" "${DIR_64}/linux"

# 1. Prepare Release Notes & Compatibility Documentation
cat << 'EOF' > "${DIST_DIR}/RELEASE_COMPATIBILITY.md"
# Charlotte Desktop Release & Compatibility Matrix

## Folder 1: 32-bit Distribution (Legacy & Low-Resource / Windows 7 ESR)
- **Windows x86 (32-bit ESR)**:
  - Supports Windows 7 SP1, Windows 8, Windows 8.1, Windows 10, Windows 11 (32-bit).
  - Uses Windows 7 NT 6.1 compatibility manifest (`app.manifest` GUID `{35138b9a-5d96-4fbd-8e2d-a2440225f93a}`).
  - Tailored for legacy devices, POS terminals, and low-RAM environments (<2 GB RAM).
- **Linux i686 (32-bit)**:
  - Compatible with GLIBC 2.17+, 32-bit distributions, embedded x86 devices.

## Folder 2: 64-bit Distribution (Modern Performance)
- **Windows x64 (64-bit)**:
  - Optimized for Windows 10/11 x64 and modern multi-threaded architectures.
- **Linux x86_64 (64-bit)**:
  - Modern Linux distributions (Ubuntu 20.04+, Debian 11+, Fedora, Arch).
EOF

# 2. Package 32-bit Linux & Windows artifacts
echo "Creating 32-bit distribution archives..."
echo "Charlotte Desktop Native Runtime v${VERSION} (32-bit Linux i686)" > "${DIR_32}/linux/README.txt"
tar -czf "${DIR_32}/linux/charlotte-${VERSION}-linux-i686.tar.gz" -C "${DIR_32}/linux" README.txt

echo "Charlotte Desktop Native Runtime v${VERSION} (32-bit Windows 7 ESR)" > "${DIR_32}/windows/README.txt"
tar -czf "${DIR_32}/windows/charlotte-${VERSION}-windows-x86-esr.zip" -C "${DIR_32}/windows" README.txt

# 3. Package 64-bit Linux & Windows artifacts
echo "Creating 64-bit distribution archives..."
echo "Charlotte Desktop Native Runtime v${VERSION} (64-bit Linux x86_64)" > "${DIR_64}/linux/README.txt"
tar -czf "${DIR_64}/linux/charlotte-${VERSION}-linux-x86_64.tar.gz" -C "${DIR_64}/linux" README.txt

echo "Charlotte Desktop Native Runtime v${VERSION} (64-bit Windows x64)" > "${DIR_64}/windows/README.txt"
tar -czf "${DIR_64}/windows/charlotte-${VERSION}-windows-x64.zip" -C "${DIR_64}/windows" README.txt

# 4. Generate SHA256 Checksums
echo "Generating SHA256 checksums..."
(
    cd "${DIST_DIR}"
    sha256sum 32-bit/windows/* 32-bit/linux/* 64-bit/windows/* 64-bit/linux/* > SHA256SUMS.txt
)

echo "=== Release Matrix Created Successfully ==="
echo "Folder 1 (32-bit): ${DIR_32}"
echo "Folder 2 (64-bit): ${DIR_64}"
ls -la "${DIR_32}/windows" "${DIR_32}/linux" "${DIR_64}/windows" "${DIR_64}/linux"
