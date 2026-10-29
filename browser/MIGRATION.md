# Native Rust + CEF conversion status

The browser source tree is now a Cargo crate. The Electron main/preload/React
renderer source, package manifests and Forge/Vite tooling were removed from
the active repository tree; they remain recoverable in Git history.

**Supported baseline, not yet runtime-certified:**
- CEF native browser window and renderer/helper process
- Rust toolbar, tabs, address input, sidebar, chat/AI provider calls
- SQLite bookmarks, history, downloads and session snapshots
- Supabase auth transport and OS keychain token persistence
- Rust request filtering and preliminary cosmetic filters
- Per-site media permission records and native settings CLI
- Rust-backed text translation API

**Known feature gaps vs the previous Electron build:**
- Full voice/audio capture, Whisper transcription and live subtitles
- MCP tool protocol and agent orchestration parity
- Screen capture, AI multimodal attachments and translation overlays
- Graphical settings, full onboarding, search-in-page and finer UI behavior
- Native extensions, provider streaming and platform-grade window polish

**Validation blocker:** Rust/CEF CI jobs have previously ended without a
runner being allocated or any compilation step executed. This does not count
as a successful compile. Before a release, run `make test`, `make build`
and the CEF smoke scripts on Linux/macOS/Windows; fix compiler/runtime
issues; verify all retained browser flows, then publish native artifacts.
