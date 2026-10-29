# Charlotte source languages

The active browser and chat-server implementations have been moved to
Rust. The website remains Next.js/TypeScript.

| Language | Source | Purpose |
| --- | --- | --- |
| **Rust** | `browser/src/`, `server/src/` | Native desktop/CEF browser, HTTP/gRPC server, chat, accounts, profile and data logic |
| **TypeScript / TSX** | `website/src/`, `website/next.config.ts` | Static Next.js website |
| **Protocol Buffers** | `server/proto/` | gRPC chat service definitions |
| **CSS** | `website/src/` | Website styling; native browser UI CSS is generated in Rust |
| **JavaScript / MJS** | `scripts/`, `website/` | Repository checks and website tooling |
| **Shell / PowerShell** | `browser/scripts/`, `server/scripts/` | Native build, package, platform smoke tests |
| **Python** | `browser/scripts/bump-version.py` | Version metadata updates |
| **Inno Setup** | `browser/packaging/windows/` | Optional Windows installer |

Build and test browser with `cd browser && make test && make build`.
Cross-target rules are listed by `make help`. Build and test the server
with `cd server && cargo test --all-targets`. Website checks run using
`cd website && npm ci && npm run build`.

GitHub's language percentages are derived from stored repository source
and are not a measure of runtime correctness. The removed Electron/TSX
implementation is still accessible in Git history for reference.

**Cross-platform status:** no successful native CEF smoke tests have
been verified. Do not treat the Rust code conversion or file removal as
proof of complete feature parity.
