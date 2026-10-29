# Electron to Rust + CEF — PR #4 migration tracker

| Former Electron subsystem | Native Rust / CEF status |
| --- | --- |
| Browser process and rendering | CEF Views Chrome runtime integrated, not yet smoke-tested |
| New-tab/start page | Rust-generated Charlotte HTML/CSS, activated for fresh profiles; visual tests pending |
| Browser UI, navigation and tabs | Custom CEF Views tabstrip and Rust command service, live tab selection/creation/closure/navigation, full startup tab URL/order/active-index restoration (untested) |
| Navigation shortcuts | Rust CEF keyboard handler: back/forward/reload and bookmarking |
| Popup/browser lifecycle | Rust window delegates connected |
| OpenShield network rules | CEF resource handler connected |
| Cosmetic filters | CEF load handler applies selector-based hiding (preliminary) |
| Scriptlets | Not wired |
| Bookmarks, folders, history | SQLite storage, legacy JSON import; native Ctrl/Cmd+D toggle |
| Chat transport | Rust chat HTTP/SSE client connected to native CEF sidebar; page-specific room reconnect and SQLite chat message persistence |
| AI providers | OpenAI/Claude/Gemini/Ollama Rust adapters connected to CEF sidebar via background Rust worker; multimodal and provider streaming pending |
| API credential storage | OS keyring module and hidden-input native CLI; no key material in browser UI |
| Custom toolbar/chat/sidebar/settings | Rust-rendered toolbar/URL field, CEF page viewport, Rust-rendered sidebar and settings CLI; advanced former Electron UI still incomplete |
| Downloads | CEF save dialog, completed/cancelled/interrupted SQLite history; graphical UI pending |
| Browser permissions | Per-origin microphone/camera policy with explicit grant CLI; graphical prompt pending |
| Native user preferences | Rust settings CLI for shields, media permissions, bookmarks, history and download records |
| Account login, voice, MCP, page translation | Not ported |
| Platform packaging | Linux/macOS/Windows packagers and runtime smoke scripts added; no runner has executed tests |

Do not delete the existing Electron implementation or merge PR #4 until
full native functionality is demonstrably equivalent and tested.

## Validation gates required before shipping

1. `cargo test --all-targets` and `cargo build --release --bins` succeed on every target.
2. CEF bundle loads its renderer/helpers and exposes the native toolbar/sidebar.
3. UI commands create, select, close, navigate and restore multiple tabs and persist the active index.
4. AI keys remain in OS keychain and no untrusted page can forge native UI actions.
5. Chat SSE stops on URL changes; original account, MCP, voice, translation and screenshot features are either ported or explicitly excluded from a release.

CI runs currently fail with zero steps/runner assignments, so no platform
passes have been recorded. Preserve the old Electron project for feature
comparison and rollback until the application has passed acceptance.
