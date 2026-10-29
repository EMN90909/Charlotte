# Charlotte Native — Rust + CEF

A CEF-based native browser process with no Electron or Node at runtime.
**PR #4 is not production-ready**. Do not merge or release before native
compilation, application smoke tests and feature-parity verification.

## Native features implemented in this branch
- CEF Chrome-style window, browser UI, popup lifecycle and browser-process callbacks
- Network request filtering via the Rust OpenShield-compatible ABP engine
- Rust SQLite storage for settings, bookmarks, folders, history and messages
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
- Cosmetic ad blocking and dynamic filter subscriptions
- Downloads, browser permissions, translations, screenshots and settings parity
- Platform-specific macOS menu/event loop, Windows installer/sandbox, Linux lifecycle
- Reproducible release builds, security regression tests, visual/browser E2E tests

The CEF/Chromium engine is native, but the full Charlotte feature set is
not yet ported. Keep the Electron build until the migration passes acceptance.
