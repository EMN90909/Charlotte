# Charlotte Browser — Rust + CEF

Charlotte's browser source, desktop host and build system are written in **Rust**.
The desktop application uses Chromium Embedded Framework (CEF) as its renderer.
Electron, React, Node.js, TypeScript, Vite and npm are not required by this
browser crate.

## Source layout

- `Cargo.toml`, `Makefile`: native Rust build targets at browser root
- `src/app.rs`, `src/cef_bridge.rs`: CEF host, browser and event lifecycle
- `src/ui_server.rs`: Rust-rendered toolbar, tabstrip, AI and page-chat sidebar
- `src/storage.rs`, `src/session.rs`, `src/tabs.rs`: SQLite state and sessions
- `src/shield.rs`: ad blocking; `src/permissions.rs`: origin permissions
- `src/auth.rs`: Supabase account transport, keychain session storage
- `src/translate.rs`: AI-backed text translation
- `src/bin/`: settings utility and CEF renderer helper
- `scripts/`, `packaging/`, `resources/`: native CEF distribution assets

## Build and test

```sh
cd browser
make check
make test
make build
make linux-x64
make linux-arm64
make windows-x64
make windows-arm64
make macos-x64
make macos-arm64
```

Cross-OS compilation requires the appropriate native linker, Rust targets
and a matching CEF distribution. `make package` builds the CEF bundle for
the **host** OS, with `CEF_PATH` set to its binary distribution. Run
`make help` for the other supported targets.

The browser starts a loopback Rust UI service, renders the controls with CEF,
and runs AI/chat network calls from Rust background workers. User secrets use
the OS keychain; profile settings and browsing history use SQLite.

**Release status:** Structural replacement of Electron has occurred, but
feature-by-feature parity with the previous Electron application and
successful Linux/macOS/Windows native compilation and runtime verification
have **not** been established. See [MIGRATION.md](MIGRATION.md).
In particular, voice capture, screenshots, advanced MCP, subtitle and
translation UI, and additional settings features are not fully connected.

The source history retains the prior Electron implementation for reference;
a Git revert remains possible. No legacy Node runtime ships through this
Cargo-based browser build.

## Licensing

The original work's MIT license (copyright 2026 Samuel Abolo) is preserved
in [LICENSE](LICENSE), with Noethric's separate rights notice in
[COMMERCIAL-LICENSE.md](COMMERCIAL-LICENSE.md).
