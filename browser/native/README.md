# Charlotte Native — Rust + CEF

A CEF-based native browser process with no Electron or Node at runtime.
**PR #4 is not production-ready**. Do not merge or release before native
compilation, application smoke tests and feature-parity verification.

## Native features implemented in this branch
- CEF Chrome-style window, browser UI, popup lifecycle and browser-process callbacks
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
- Rust chat connect, send, poll and SSE transport clients
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
- Charlotte custom toolbar, chat sidebar and AI UI wired to typed CEF process messages
- Full provider streaming, multimodal chat, voice and MCP workflows
- Complete cosmetic/scriptlet compatibility and dynamic filter subscriptions
- Native permission prompts/settings UI (media CLI works; geolocation and
  other Chrome permission prompts remain native), translation, screenshots and
  full graphical downloads/bookmarks/settings interfaces
- Multi-tab CEF view switching and complete startup restoration of the saved tab list
- Platform-specific macOS menu/event loop, Windows installer/sandbox, Linux lifecycle
- Reproducible release builds, security regression tests, visual/browser E2E tests

The CEF/Chromium engine is native, but the full Charlotte feature set is
not yet ported. Keep the Electron build until the migration passes acceptance.
