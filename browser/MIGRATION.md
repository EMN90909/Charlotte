# Electron → Rust migration map

| Electron area | Rust/CEF owner |
| --- | --- |
| `index.ts`, `windowManager.ts` | `src/app.rs` and CEF Views |
| `tabManager.ts` | CEF Chrome runtime tab model |
| Electron IPC/preloads | CEF process messages behind typed Rust handlers |
| session/profile stores | `src/profile.rs`, future `charlotte-core` |
| request interception / OpenShield | future `charlotte-shield` CEF request handler |

The Electron implementation remains on `main` until this PR merges. It is never invoked by this Rust build, preventing a data-loss migration from being bundled with the engine change.
