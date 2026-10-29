# Electron to Rust + CEF — PR #4 migration tracker

| Former Electron subsystem | Native Rust / CEF status |
| --- | --- |
| Browser process and rendering | CEF Views Chrome runtime integrated, not yet smoke-tested |
| Browser UI, navigation and tabs | CEF Chrome-style UI, Charlotte custom chrome pending |
| Popup/browser lifecycle | Rust window delegates connected |
| OpenShield network rules | CEF resource handler connected |
| Cosmetic filters | CEF load handler applies selector-based hiding (preliminary) |
| Scriptlets | Not wired |
| Bookmarks, folders, history | SQLite storage, legacy JSON import |
| Chat transport | Rust connect/send/poll/SSE client modules; UI pending |
| AI providers | Rust HTTP adapters; UI and streaming pending |
| API credential storage | OS keyring module; UI pending |
| Custom toolbar/chat/sidebar/settings | Not ported |
| Downloads, account login, voice, MCP, page translation | Not ported |
| Platform packaging | Scaffolding present; smoke/installer validation pending |

Do not delete the existing Electron implementation or merge PR #4 until
full native functionality is demonstrably equivalent and tested.
