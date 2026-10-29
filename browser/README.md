# Charlotte Browser — Rust + CEF

Charlotte's browser source, desktop host and build system are written in **Rust**.
The desktop application uses Chromium Embedded Framework (CEF) as its renderer.
Electron, React, Node.js, TypeScript, Vite and npm are not required by this
browser crate.

## Charlotte Auto — one task, the right capability

**Auto is the default option** in Charlotte's Rust sidebar. The chosen AI
provider first decides whether the request can be answered directly, read
locally with a deterministic Chromium skill, needs an AI explanation of the
active page, requires the bounded Chromium interaction agent, uses a
configured MCP service, or should hand off browser data to an MCP tool.
The Rust host checks the selected route against the actual active tab and
the user's enabled app list; the AI cannot invent capabilities.

A user can configure OpenAI, Claude, Gemini or local Ollama via a signed
**AI provider connection** form. API keys remain in the OS credential store
rather than a webpage or SQLite. Users can set model IDs and explicitly
test a saved model (potential API charges apply).

Auto only gives the model the active tab **origin** for initial routing.
Before it reads and sends current webpage content, the user must approve
a separate page-sharing consent. Every website modification and MCP tool
call retains its own per-step approval. A Chromium-first task can pause
and request fresh permission to transfer the resulting page to a connected
app, then show the actual proposed MCP operation for approval. Local
read-only browser skills keep page content on device.

For workflows, permission boundaries and non-claims see
[AUTO_ORCHESTRATION.md](AUTO_ORCHESTRATION.md). This is source code being integrated into the main branch, **not a verified production build** or a hosted multi-tenant system.

## Charlotte MCP integration directory

Charlotte now includes **43 opt-in MCP connection profiles** from the supplied
33-group integration directory. The sidebar allows choosing a provider or
entering a verified custom HTTPS endpoint; the settings CLI offers catalog
listing, enable/disable, credential setup, explicit OAuth 2.1 PKCE
authorization, and a connection/tool discovery check. Optional, separately
installed local MCP tools use allowlisted STDIO executables; a native SQLite
adapter supplies strictly read-only access to one user-selected database.

No third-party accounts are connected automatically. Bearer/OAuth tokens
are stored in the OS credential store, all tool calls still require explicit
approval, and no more than five relevant saved MCP profiles are contacted
per planning request. Vendor availability, OAuth client registration, package
installation and remote API pricing vary. See the
[MCP catalog and setup guide](MCP_CATALOG.md) for the 43 individual profiles,
setup commands and release caveats.

## Source layout

- `Cargo.toml`, `Makefile`: native Rust build targets at browser root
- `src/app.rs`, `src/cef_bridge.rs`: CEF host, browser and event lifecycle
- `src/ui_server.rs`, `src/orchestrator.rs`: AI routing, permissions, and Rust-rendered tab/agent UI
- `src/storage.rs`, `src/session.rs`, `src/tabs.rs`: SQLite state and sessions
- `src/shield.rs`: ad blocking; `src/permissions.rs`: origin permissions
- `src/auth.rs`: Supabase account transport, keychain session storage
- `src/translate.rs`: AI-backed text translation
- `src/browser_automation.rs`, `src/browser_agent.rs`: native CEF/CDP browser automation and bounded AI action loop
- `src/bin/`: settings utility and CEF renderer helper
- `scripts/`, `packaging/`, `resources/`: native CEF distribution assets

## Build and test

The simplest host-native entry points are directly inside `browser/`:

**Windows (CMD):**

```bat
cd browser
set "CEF_PATH=C:\\path\\to\\cef_binary_distribution"
build.bat
build.bat --verify
```

**Linux / macOS (shell):**

```sh
cd browser
export CEF_PATH="/path/to/cef_binary_distribution"
sh build.sh
sh build.sh --verify
```

Both launchers support `--check`, `--test`, `--compile-only`,
`--bundle` (default), `--verify` and `--help`. Their default produces
a native CEF bundle under `browser/target/bundle` and checks that
`CEF_PATH` and `bundle-cef-app` are present. The raw Rust executable alone
is not a runnable browser without the Chromium distribution.

[Full build prerequisites and troubleshooting](BUILDING.md).

Power users can also use the existing `Makefile` targets:

```sh
make check
make test
make build
make package
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
In particular, voice capture, screenshots, autonomous multi-step MCP, subtitle and
translation UI, and additional settings features are not fully connected.

The source history retains the prior Electron implementation for reference;
a Git revert remains possible. No legacy Node runtime ships through this
Cargo-based browser build.

## Charlotte — Chromium AI browser automation

Select **Automate this Chromium tab** in Charlotte's sidebar, describe a
goal and let the Rust agent inspect the active page, propose one action,
obtain your explicit approval, execute it using CEF's built-in Chrome
DevTools Protocol bridge, and verify the result.

The extended automation layer now inspects **open shadow roots** and
**same-origin iframe documents**, and provides mouse-based drag/drop
between observed controls. A separate **Visual automation (shares a
screenshot)** mode captures a bounded JPEG of the visible Chromium
page and sends it to the chosen AI provider; it can propose approved
coordinate clicks on visually rendered content, including a cross-origin
iframe's visible pixels **without reading its isolated DOM**.
JavaScript alert/confirm/prompt dialogs can be resolved explicitly
through Charlotte's sidebar. System file pickers and permission dialogs
remain under direct user control.

### Accessibility-first agent control and task recovery

Based on the supplied agentic-browser research, Charlotte now requests a
bounded Chromium Accessibility tree, merges semantic roles/names with the
visible control inventory and gives the model **numbered element tools**:
click_index, type_index and drag_index. The native Rust controller resolves
these ephemeral IDs to live DOM elements and still requires explicit approval.

A stale selector can only be re-targeted when a fresh observation finds
one uniquely matching accessible role/name. The replacement **must be
approved again** rather than quietly applying the older consent.

Automation tasks are checkpointed in SQLite WAL with a bounded local log.
The sidebar can **Inspect and re-plan saved task** after a restart, or
**Discard saved task**. Nothing previously attempted is automatically
replayed. **Pause / Take Over** stops future actions, although a Chromium
action already dispatched may still finish. Checkpoints do not contain
screenshots, field values, credentials or pending approval tokens.

See [AUTOMATION_RESEARCH_INTEGRATION.md](AUTOMATION_RESEARCH_INTEGRATION.md)
for the full mapping of the provided Strawberry-style architecture research
to Charlotte's implemented features, tradeoffs and outstanding gaps.
Parallel autonomous tabs, custom skills/routines, OOPIF node handles and
opt-in long-term semantic memory are **not** completed.

Charlotte also has **built-in read-only browser skills** in the sidebar:
**Page summary**, **Visible links**, **Interactive controls**, and **Frame
inventory**. These deterministic Rust recipes inspect the current CEF tab
without an AI call and display results locally. They never put the resulting
page text into AI chat history. Results are cleared when you switch tabs or
navigate away. They are not general-purpose unattended workflow recording.

For the AI automation loop, changing tabs, closing the target tab, leaving
the authorized origin or a Chromium renderer crash **pauses the task,
invalidates pending approval, and requires manual inspection and re-planning**.
CEF reports renderer termination to Rust rather than silently retrying a
possibly completed action. Native cross-platform verification is still
required.

See [AGENTIC_BROWSER_RESEARCH.md](AGENTIC_BROWSER_RESEARCH.md) for the implementation gap analysis and release gates derived from the supplied browser automation research.

See [ADVANCED_AUTOMATION.md](ADVANCED_AUTOMATION.md) for scope, consent,
privacy, iframe isolation, unverified behaviors and native test cases. Supported first-pass
actions include page inspection, same-origin navigation, real Chromium
mouse clicks, non-sensitive text entry and scrolling, for up to eight
approved steps. There is no Selenium, external Chrome, Node or browser
debugging port.

The page observation (bounded visible page text and interactive controls)
is sent to your selected AI provider **only after you choose browser
automation mode**. In visual mode, visible screenshot pixels are
additionally sent. Password and file inputs are excluded from control
inspection. Browser actions remain scoped to one active tab and require
explicit approval. Direct DOM access to cross-origin frames and automated
operating-system dialogs are not supported; native runtime behavior
has not yet been verified.

See [BROWSER_AUTOMATION.md](BROWSER_AUTOMATION.md) for the Chromium/CEF
research, implementation, security decisions, limitations and validation
matrix. Compilation and end-to-end tests remain unverified.

## Charlotte AI Agent, MCP and site notifications

Charlotte's native Rust assistant is called **Charlotte**. The sidebar can
connect to explicitly registered MCP servers, propose a tool use and ask the
user to **Approve once** or **Deny** the exact call before execution. Responses
from external tools are treated as untrusted data. MCP credentials are kept
in the OS keychain, not webpage JavaScript.

Inactive tabs enter **eco sleep** (native view occlusion); their pages remain
alive so music, media and service-worker contexts are not deliberately cut off.
Deep page discard is intentionally disabled because it would break
uninterrupted background activity. ☾ identifies inactive eco-sleep tabs.

Site-by-site **Notifications** Allow/Block/Reset controls are available in the
sidebar and the native settings CLI. These decisions are enforced through
CEF's permission callback. Full remote **Web Push delivery and OS display**
are **not yet certified**: upstream CEF issue #4138 describes GCM/PushManager
subscription failures, and native platform popup integration still needs
end-to-end verification.

See [CHARLOTTE_AGENT.md](CHARLOTTE_AGENT.md) for setup commands, scope,
security design, and the outstanding Web Push limitation.

## Charlotte Shields (native Rust ad blocker)

Charlotte Shields is built **into the Rust browser binaries** using the
Brave adblock-rust crate; it does not require OpenShield-js, Node, a browser
extension, or a second adblock executable. Network filtering runs in CEF
resource callbacks. Cosmetic filters run in each CEF page frame using
selectors generated by the Rust engine, without evaluating remote scriptlets.

- A visible `🛡 ON / OFF` toolbar button changes network and cosmetic
  filtering immediately, persists the setting and displays blocked requests.
- Offline starter rules are always included so protection works on first launch.
- Default subscriptions are **EasyList** and **EasyPrivacy**. Six additional
  maintained subscriptions follow OpenShield-js's named-list model.
- The first launch downloads enabled lists on a background thread; cached
  lists are refreshed no more than once per 24 hours and remain usable offline.
  Downloads use HTTPS, connection/time limits and an 8 MiB cap per list.
- Edit `<profile>/shield-lists/custom.txt` for user-maintained ABP/uBO rules.
  This file is local-only and is never fetched from the web.
- `charlotte-settings shields lists` shows available subscriptions.
  `charlotte-settings shields list on|off <list-id>` enables optional lists.
  `charlotte-settings shields update` refreshes enabled subscriptions.
  CLI list changes need a browser restart; toolbar toggling does not.

The browser and filter engine are linked as part of the same Cargo build.
**CEF itself remains a native Chromium runtime**, so packaging also requires
the platform-matched CEF shared library, subprocess helpers, and resource
files. "One build" means one Charlotte application bundle, not a single
statically linked executable.

Build status must be determined from actual target-OS compilation and the CEF
runtime smoke tests. The current source tree alone is not proof that all
platform builds or browser navigation work.

See [PERFORMANCE.md](PERFORMANCE.md) for the Rust/CEF threading contract, release build flags, lazy tab loading, security constraints and reproducible Linux native memory/startup measurements (`make measure`).

## Licensing

The original work's MIT license (copyright 2026 Samuel Abolo) is preserved
in [LICENSE](LICENSE), with Noethric's separate rights notice in
[COMMERCIAL-LICENSE.md](COMMERCIAL-LICENSE.md).
