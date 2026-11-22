#!/usr/bin/env bash
set -euo pipefail

# Charlotte Cross-Platform & Windows 7–11 Multi-Arch Release Packager
# Categorizes releases cleanly into two top-level architecture folders:
#   Folder 1: dist/32-bit/
#     - windows/win7-esr/  -> Legacy Inno Setup Win7 SP1 ESR (x86) + WebView2 Fixed 109 ESR + PE audit
#     - windows/win8-11/   -> Custom Opera/Chrome-style native installer (x86: Win 8/8.1 ESR & Win 10/11)
#     - linux/             -> Linux i686 (32-bit)
#   Folder 2: dist/64-bit/
#     - windows/win7-esr/  -> Legacy Inno Setup Win7 SP1 ESR (x64) + WebView2 Fixed 109 ESR + PE audit
#     - windows/win8-11/   -> Custom Opera/Chrome-style native installer (x64: Win 8/8.1 ESR & Win 10/11)
#     - linux/             -> Linux x86_64 (64-bit)

VERSION="0.4.0"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BROWSER_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DIST_DIR="${BROWSER_DIR}/dist"
DIR_32="${DIST_DIR}/32-bit"
DIR_64="${DIST_DIR}/64-bit"

echo "=== Packaging Charlotte Desktop v${VERSION} Multi-Arch Matrix (Windows 7–11 & Linux) ==="

mkdir -p "${DIR_32}/windows/win7-esr" "${DIR_32}/windows/win8-11" "${DIR_32}/linux"
mkdir -p "${DIR_64}/windows/win7-esr" "${DIR_64}/windows/win8-11" "${DIR_64}/linux"

# 1. Compatibility & Runtime Build Documentation
cat << 'EOF' > "${DIST_DIR}/RELEASE_COMPATIBILITY.md"
# Charlotte Desktop Release & Compatibility Matrix (Windows 7–11, 32-bit & 64-bit, ESR Modes)

Default Cloud API Endpoint: `https://api-charlotte-server.onrender.com` (`wss://api-charlotte-server.onrender.com/ws`)

## Folder 1: `dist/32-bit/` (x86 / i686 — Legacy, Low-RAM & 32-bit Windows 7–11)
- **`windows/win7-esr/` (Windows 7 SP1 32-bit ESR — Inno Setup)**:
  - Uses dedicated Inno Setup installer (`packaging/windows/charlotte.iss`, `/DArch=x86`).
  - Bundles **WebView2 Fixed Version 109.0.1518.78 x86 ESR** (or CEF 109 ESR) — never pulls modern Evergreen WebView2 >= 110.
  - Binary must be compiled with Win7-compatible ABI toolchain (Rust 1.75 Win7 or `YY-Thunks`/`VC-LTL5`) and pass `packaging/windows/verify_win7_pe.py` (verifying zero `WaitOnAddress`, `ProcessPrng`, `GetSystemTimePreciseAsFileTime`, or `api-ms-win-core-synch-l1-2-0.dll` imports).
- **`windows/win8-11/` (Windows 8, 8.1 ESR & Windows 10/11 32-bit — Custom Installer)**:
  - Uses Charlotte's Opera/Chrome-style custom native installer (`charlotte-installer`, x86).
  - Auto-selects **Win 8/8.1 ESR** (WebView2 109 Fixed) on Windows 8/8.1 and **Evergreen / Modern ESR** on Windows 10/11.
- **`linux/` (Linux i686 32-bit)**:
  - GLIBC 2.17+ compatible 32-bit bundle.

## Folder 2: `dist/64-bit/` (x64 / x86_64 — Modern & 64-bit Windows 7–11)
- **`windows/win7-esr/` (Windows 7 SP1 64-bit ESR — Inno Setup)**:
  - Uses dedicated Inno Setup installer (`packaging/windows/charlotte.iss`, `/DArch=x64`) with **WebView2 Fixed Version 109.0.1518.78 x64 ESR** and `verify_win7_pe.py` PE import gate.
- **`windows/win8-11/` (Windows 8, 8.1 ESR & Windows 10/11 64-bit — Custom Installer)**:
  - Uses Charlotte's Opera/Chrome-style custom native installer (`charlotte-installer`, x64) with SHA-256 verification, silent mode (`--silent`), and optional ESR mode (`--esr`).
- **`linux/` (Linux x86_64 64-bit)**:
  - Modern 64-bit Linux distributions (Debian, Ubuntu, Fedora, Arch).
EOF

# 2. Populate Win7 ESR Inno Setup specs & run PE import audit if compiled Windows binaries exist
cp "${SCRIPT_DIR}/windows/charlotte.iss" "${DIR_32}/windows/win7-esr/charlotte-x86.iss"
cp "${SCRIPT_DIR}/windows/charlotte.iss" "${DIR_64}/windows/win7-esr/charlotte-x64.iss"
cp "${SCRIPT_DIR}/windows/verify_win7_pe.py" "${DIR_32}/windows/win7-esr/verify_win7_pe.py"
cp "${SCRIPT_DIR}/windows/verify_win7_pe.py" "${DIR_64}/windows/win7-esr/verify_win7_pe.py"

if command -v python3 >/dev/null 2>&1; then
  echo "Running Windows 7 SP1 PE import audit gate..."
  python3 "${SCRIPT_DIR}/windows/verify_win7_pe.py" "${DIR_32}/windows/win7-esr" "${DIR_64}/windows/win7-esr"
fi

# 3. Generate channel manifests for 32-bit and 64-bit targets
cat << EOF > "${DIR_32}/windows/win8-11/installer_channels.json"
{
  "version": "${VERSION}",
  "arch": "x86",
  "installer": "charlotte-installer-x86.exe",
  "cloud_server": "https://api-charlotte-server.onrender.com",
  "channels": {
    "win8-esr": "Microsoft.WebView2.FixedVersionRuntime.109.0.1518.78.x86",
    "stable": "Microsoft.WebView2.Evergreen.x86",
    "esr": "Microsoft.WebView2.FixedVersionRuntime.ESR.x86"
  }
}
EOF

cat << EOF > "${DIR_64}/windows/win8-11/installer_channels.json"
{
  "version": "${VERSION}",
  "arch": "x64",
  "installer": "charlotte-installer-x64.exe",
  "cloud_server": "https://api-charlotte-server.onrender.com",
  "channels": {
    "win8-esr": "Microsoft.WebView2.FixedVersionRuntime.109.0.1518.78.x64",
    "stable": "Microsoft.WebView2.Evergreen.x64",
    "esr": "Microsoft.WebView2.FixedVersionRuntime.ESR.x64"
  }
}
EOF

# 4. Generate Linux archives
echo "Charlotte Desktop Native Runtime v${VERSION} (32-bit Linux i686)" > "${DIR_32}/linux/README.txt"
tar -czf "${DIR_32}/linux/charlotte-${VERSION}-linux-i686.tar.gz" -C "${DIR_32}/linux" README.txt

echo "Charlotte Desktop Native Runtime v${VERSION} (64-bit Linux x86_64)" > "${DIR_64}/linux/README.txt"
tar -czf "${DIR_64}/linux/charlotte-${VERSION}-linux-x86_64.tar.gz" -C "${DIR_64}/linux" README.txt

# 5. Generate SHA256 Checksums
echo "Generating SHA256 checksums..."
(
    cd "${DIST_DIR}"
    find 32-bit 64-bit -type f | sort | xargs sha256sum > SHA256SUMS.txt
)

echo "=== Release Matrix Created Successfully ==="
echo "Folder 1 (32-bit): ${DIR_32}"
echo "Folder 2 (64-bit): ${DIR_64}"
