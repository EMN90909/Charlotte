# Charlotte Native — Rust + CEF

A CEF-based native browser process with no Electron or Node at runtime.
**PR #4 is not production-ready**. Do not merge or release before native
compilation, application smoke tests and feature-parity verification.

## Native features implemented in this branch
- CEF Views desktop with three panes: Rust-hosted custom tabstrip, Chromium page viewport and Rust-hosted chat/AI sidebar
- Native CEF tab selection, close/new, navigation, popups, history, and full saved tab-order/active-tab restoration
- Local loopback Rust Axum UI service with random per-launch CSRF token, strict POST Origin checking and no Node/Electron runtime
- Rust-rendered Charlotte new-tab/start page for fresh profiles (local HTML/CSS with HTTPS search and bookmarks; no external scripts)
- Network request filtering and preliminary CSS selector hiding via the Rust ABP engine
- Rust SQLite storage for settings, bookmarks, folders, history, messages and completed downloads
- Browser lifecycle/tab state and ordered multi-tab session snapshots (full tab UI/restore pending)
- Native shortcuts: Ctrl/Cmd+D bookmarking, Alt+Left/Right, Ctrl/Cmd+R, F5 and hard reload
- CEF native Save dialog and terminal download tracking; graphical downloads panel pending
- Per-origin microphone/camera permission policy (default deny, explicit grants only)
- Local Rust settings CLI for site media permissions, shield preferences, bookmarks, history and downloads
- OS credential-store integration for AI service keys
- AI provider adapters (OpenAI, Claude, Gemini, Ollama)
- Rust chat connect, send, poll and SSE transport clients, connected to page-specific sidebar chat
- AI provider actions connected to Rust sidebar (keys loaded from OS keychain; calls executed on worker threads)
- AI conversation history persisted to SQLite; sidebar updates automatically without interrupting focused inputs
- macOS renderer helper and Windows sandbox bootstrap entry
- Explicit one-time migration of Electron bookmarks and browsing history

## Build
Rust, C/C++ tooling and the matching CEF 156.2 distribution are required.
Follow https://github.com/tauri-apps/cef-rs for CEF installation and bundling.

From browser/native run:

    cargo test --lib
    cargo build --release --bins

Native settings and permissions utility (run from browser/native):

    cargo run --bin charlotte-settings -- --help
    cargo run --bin charlotte-settings -- permission allow camera https://meet.example
    cargo run --bin charlotte-settings -- permission allow microphone https://meet.example
    cargo run --bin charlotte-settings -- bookmark list
    cargo run --bin charlotte-settings -- downloads list
    cargo run --bin charlotte-settings -- shields off

Remembered media grants apply only to matching secure origins; browser camera and
microphone requests are denied by default. Reload/restart the native app after
changing the shield preference.

For packaging, install the appropriate CEF bundler:

    cargo install cef --version '156.2.0+156.0.14' --features build-util --bin bundle-cef-app

Set CEF_PATH to the matching CEF distribution and run scripts/build.sh
(Linux/macOS) or scripts/build.bat (Windows).

For data migration, set CHARLOTTE_IMPORT_ELECTRON_DIR to the original
Electron userData folder when launching Charlotte. This imports only
bookmarks.json and history.json; source data is never deleted.
Passwords, cookies and API credentials must be re-authorized in the new app.

## Outstanding before complete Electron replacement
- Verify and polish Charlotte's custom UI layout, richer agent settings and per-page sidebar navigation; the Rust-controlled loopback form bridge is in place
- Full provider streaming, multimodal chat, voice and MCP workflows
- Complete cosmetic/scriptlet compatibility and dynamic filter subscriptions
- Native permission prompts/settings UI (media CLI works; geolocation and
  other Chrome permission prompts remain native), translation, screenshots and
  full graphical downloads/bookmarks/settings interfaces
- Full browser parity beyond the newly wired core (advanced downloads, audio/transcription, subtitles, account authentication, custom visual design and other original Electron controls)
- Platform-specific macOS menu/event loop, Windows installer/sandbox, Linux lifecycle
- Reproducible release builds, security regression tests, visual/browser E2E tests

The CEF/Chromium engine is native, but the full Charlotte feature set is
not yet ported. Keep the Electron build until the migration passes acceptance.

## Native sidebar + key management

The top toolbar and chat sidebar are rendered by CEF from a Rust server bound
to `127.0.0.1` on an ephemeral port. POST actions require a 256-bit
per-launch token, matching Host and same-origin Origin. External websites
receive no native OS bridge. AI secrets are obtained only by the Rust host
from the system credential store. Configure them without typing tokens into
command-line arguments:

```sh
cd browser/native
cargo run --bin charlotte-settings -- credential set openai
cargo run --bin charlotte-settings -- credential set claude
cargo run --bin charlotte-settings -- credential set gemini
cargo run --bin charlotte-settings -- credential show openai
```

The sidebar can connect to a Charlotte chat service by server URL and
username; chat SSE is stopped and reauthentication required when the active
page changes. AI turns are saved locally to SQLite; do not put API credentials
in the browser's SQLite settings.

## Cross-platform acceptance checks

The native CI and release workflows contain Linux, macOS and Windows runtime
smoke gates that build the matching CEF binary bundle, launch the desktop app,
check the native tabstrip/sidebar HTTP endpoints, and confirm a tokenless
command POST is denied. Linux requires Xvfb; macOS uses the .app executable;
Windows uses the bundled Charlotte.exe.

```sh
cd browser/native
cargo test --all-targets
cargo build --release --bins
# After installing export-cef-dir and bundle-cef-app and setting CEF_PATH:
sh scripts/build.sh
bash scripts/smoke-native.sh
```

**Verification caveat:** The GitHub Actions runs seen during development
failed before runner allocation; these checks have not executed successfully.
Do not interpret the presence of test scripts as proof of cross-platform
functional parity, or publish a production release without passing them.
