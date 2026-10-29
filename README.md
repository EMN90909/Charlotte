# Charlotte

Charlotte is a Noethric-branded browser with per-page chat and AI features.
This monorepo contains three independently built components:

| Directory | Implementation | Role |
| --- | --- | --- |
| [browser/](browser/) | **Rust + CEF** | Native desktop browser, tabs, chat/AI sidebar, profiles and settings |
| [server/](server/) | **Rust** | Native HTTP/gRPC chat server and ops daemon |
| [website/](website/) | **Next.js / TypeScript** | Marketing and static site |

## Browser: Rust + CEF

```sh
cd browser
make help
make check
make test
make build
```

Platform compilation targets include `make linux-x64`, `make linux-arm64`,
`make windows-x64`, `make windows-arm64`, `make macos-x64`,
`make macos-arm64`, and `make freebsd-x64`. These are Cargo target
builds, and **cross-platform compilation additionally needs the matching
linker, target runtime and CEF SDK**. For a runnable current-host CEF app,
set `CEF_PATH` and execute `make package`.

The browser's active source is `browser/src/**/*.rs`. It does not use
Electron, React, npm, TypeScript or Node.js at runtime. It uses CEF for
HTML/CSS and web-content rendering, a Rust-controlled toolbar and sidebar,
SQLite for local data, and the operating system keychain for secrets.

**Important:** The old Electron source was removed from the current tree
and is recoverable through Git history. Not all former Electron features
have functional Rust parity yet. Outstanding work includes full voice
capture/transcription, multimodal agent tools, MCP transport/agent parity,
advanced settings and screenshot/translation overlays. The native browser
has not passed cross-platform runtime certification. See
[browser/MIGRATION.md](browser/MIGRATION.md).

## Server: native Rust

```sh
cd server
cargo test --all-targets
cargo build --release --bins
CHAT_JWT_SECRET=your-strong-development-secret cargo run --bin charlotte-server
```

Default HTTP port is 8080 and gRPC is 50051. See
[server/README.md](server/README.md) and
[server/PROTOCOL.md](server/PROTOCOL.md) for deployment and protocol notes.

## Website

```sh
cd website
npm ci
npm run build
```

Only the website requires Node.js and TypeScript; it can be statically
exported and hosted without a long-running Node server.

## Native authentication

Set `CHARLOTTE_SUPABASE_URL` and
`CHARLOTTE_SUPABASE_PUBLISHABLE_KEY` to a supported Supabase project.
Charlotte's Rust account adapter stores tokens in the OS credential store;
do not provide a Supabase service-role secret or expose access tokens to
CEF webpage JavaScript.

See [LANGUAGES.md](LANGUAGES.md) for source languages and
[browser/packaging/README.md](browser/packaging/README.md) for packaging.

## Legal rights

The original project code remains subject to the 2026 MIT license
of Samuel Abolo, as preserved in [LICENSE](LICENSE). Charlotte/Noethric's
separate commercial distribution notices do not revoke or narrow
any original MIT grant. Read
[browser/COMMERCIAL-LICENSE.md](browser/COMMERCIAL-LICENSE.md).
