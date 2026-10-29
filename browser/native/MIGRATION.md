# Electron → Rust migration map

| Electron area | Rust/CEF owner |
| --- | --- |
| `index.ts`, `windowManager.ts` | `src/app.rs` and CEF Views |
| `tabManager.ts` | Not yet ported |
| Electron IPC/preloads | CEF process messages behind typed Rust handlers |
| session/profile stores | `src/profile.rs`, future `charlotte-core` |
| request interception / OpenShield | future `charlotte-shield` CEF request handler |

The Electron implementation is preserved on the draft branch until feature parity and migration tests pass. The prototype must not be merged as a replacement yet.
