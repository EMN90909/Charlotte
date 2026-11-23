# Native Charlotte Browser Packaging & Engine Routing

Charlotte is a cross-platform, multi-architecture autonomous AI browser supporting **Windows 7–11** (both 32-bit and 64-bit, with ESR modes) and **Linux** (x86_64 and i686).

Default Cloud API Gateway: `https://api-charlotte-server.onrender.com` (`wss://api-charlotte-server.onrender.com/ws`).

---

## 1. Web Engine Routing & Crates.io Architecture

Charlotte uses an extensible, platform-native web engine design (`src/engine/mod.rs` via `WebEngine` trait):

### Linux: WebKitGTK
- **Crates / Linking**: Built on system GTK 3 and WebKitGTK C libraries resolved via `pkg-config` (`gtk+-3.0`, `webkit2gtk-4.1` or `webkit2gtk-4.0`).
- **Prerequisites**:
  - Debian/Ubuntu: `sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev pkg-config build-essential`
  - Fedora: `sudo dnf install -y gtk3-devel webkit2gtk4.1-devel pkgconf`
  - Arch Linux: `sudo pacman -S --needed gtk3 webkit2gtk-4.1 pkgconf`
- **Engine Implementation**: `src/engine/webkitgtk.rs` manages the WebKit web context, page navigation, script injection, forced dark mode, and isolated sessions.

### Windows: WebView2 & COM
- **Crates / Linking**: Connects to the Microsoft Edge WebView2 runtime using COM interfaces (`WebView2Loader.dll`).
- **Windows 10 & 11**: Automatically targets the system **Evergreen WebView2** runtime.
- **Windows 7 SP1, 8, & 8.1 (ESR)**: Microsoft dropped Windows 7/8 support in Chromium/WebView2 110+. Charlotte routes to **Fixed Version 109.0.1518.78 ESR** (`dist/32-bit/windows/win7-esr/` and `dist/64-bit/windows/win7-esr/`).
- **Engine Implementation**: `src/engine/webview2.rs` coordinates COM controller initialization, navigation, and DevTools/automation script execution.

---

## 2. Multi-Architecture Output Matrix (`dist/`)

All build artifacts and installers are organized into two top-level architecture folders:

### `dist/32-bit/` (x86 / i686)
- **`windows/win7-esr/`**:
  - Inno Setup installer script (`charlotte-x86.iss`, `/DArch=x86`)
  - WebView2 Fixed Version 109.0.1518.78 x86 ESR runtime configuration
  - PE import compliance auditor (`verify_win7_pe.py`)
- **`windows/win8-11/`**:
  - Native custom installer (`charlotte-installer-x86.exe`)
  - Multi-channel selector manifest (`installer_channels.json`)
- **`linux/`**:
  - 32-bit Linux tarball (`charlotte-0.4.0-linux-i686.tar.gz`)

### `dist/64-bit/` (x64 / x86_64)
- **`windows/win7-esr/`**:
  - Inno Setup installer script (`charlotte-x64.iss`, `/DArch=x64`)
  - WebView2 Fixed Version 109.0.1518.78 x64 ESR runtime configuration
  - PE import compliance auditor (`verify_win7_pe.py`)
- **`windows/win8-11/`**:
  - Native custom installer (`charlotte-installer-x64.exe`)
  - Multi-channel selector manifest (`installer_channels.json`)
- **`linux/`**:
  - 64-bit Linux executables (`Charlotte`, `charlotte-settings`, `charlotte-installer`)
  - Desktop entry (`charlotte.desktop`) and launcher script (`charlotte.sh`)
  - 64-bit Linux tarball (`charlotte-0.4.0-linux-x86_64.tar.gz`)

---

## 3. How to Build & Package

### On Windows
```cmd
REM Build all release binaries, installer exes, and package dist\
build.bat

REM Check compilation
build.bat --check

REM Run tests
build.bat --test
```

### On Linux / macOS
```sh
# Build all release binaries, desktop shortcuts, and package dist/
sh build.sh

# Install GTK 3 and WebKitGTK dev packages on Debian/Ubuntu/Fedora/Arch
sh build.sh --install-deps

# Check compilation
sh build.sh --check

# Run tests
sh build.sh --test
```
