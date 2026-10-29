# Charlotte — Eco Sleeping, Website Notifications, and MCP AI Agent

## Inactive tabs

Charlotte's default is **eco sleep, not page discard**. The native Rust tab
model marks inactive tabs as sleeping (☾), and the CEF Views host makes only
the active tab visible. Chromium may throttle occluded painting and timers,
but its browser, audio playback, page/network contexts, and service workers
are retained. Switching tabs wakes a view immediately. Music, live calls,
video streams and unsaved form state are not explicitly paused or destroyed.

This is **not** deep memory reclamation: Chromium can still use memory in
hidden tabs. Deep freeze or unload cannot safely guarantee uninterrupted
music, notifications, live calls and web activity without reliable
activity/permission tracking and site-specific tests. Startup restoration
continues to lazily navigate previously inactive tabs.

## Website notifications

- The native CEF `on_show_permission_prompt` hook recognizes the
  version-pinned CEF NOTIFICATIONS permission flag and honors only explicit
  per-origin consent recorded in SQLite.
- In Charlotte's sidebar, the active secure website can be **Allowed**,
  **Blocked** or **Reset**. No arbitrary site can grant itself permission.
- The CLI also supports
  `charlotte-settings permission allow|deny|show|reset notifications <origin>`.
- Unknown permissions remain handled by CEF's existing policy. Camera and
  microphone permissions are separate and are not implicitly granted.
- **Important upstream limitation:** CEF's browser notification presentation
  and remote Web Push subscription/delivery are not guaranteed by the
  permission callback. In March 2026, CEF reported that disabled GCM can
  prevent `PushManager.subscribe()` from completing:
  https://github.com/chromiumembedded/cef/issues/4138
  This change does **not** pretend to provide a working proprietary push
  transport or spoof remote pushes. System popups, service worker push,
  subscription persistence and `notificationclick` must be proven on target
  systems before Web Push can be advertised as complete.

## Charlotte — native AI agent

The browser assistant is named **Charlotte**. It is written in Rust and
uses existing configured AI providers (OpenAI, Claude, Gemini and Ollama)
with a single-step, explicitly approved MCP tool workflow.

1. Configure a server from a terminal:
   `charlotte-settings mcp add docs https://example.org/mcp`.
2. If authentication is needed:
   `charlotte-settings mcp token set docs` (secret goes to the OS keychain).
3. Open Charlotte's sidebar, choose an AI provider and ask for a task.
4. Charlotte requests the tools advertised by registered MCP servers on
   a background worker and proposes at most **one** tool invocation.
5. The exact namespaced tool name and arguments are displayed. Click
   **Approve once** to execute or **Deny** to cancel; there is no auto-approval.
6. Charlotte reports the tool result back in conversational language. Tool
   descriptions and results are treated as untrusted data, not instructions.

`charlotte-settings mcp servers` lists registrations;
`charlotte-settings mcp remove docs` deletes a registration.
`charlotte-settings mcp token remove docs` deletes its access token.

The MCP client uses HTTPS (or explicit loopback HTTP), disallows redirects,
caps response size at 2 MiB, verifies JSON-RPC message IDs and checks that
any invoked tool was advertised. The UI has a CSRF token and strict Origin
checks. MCP is not allowed to run directly from arbitrary website scripts.

### Agent scope

This is an initial bounded agent tool-use loop, **not** a fully autonomous
multi-step agent or a browser automation engine. Charlotte currently proposes
one tool per turn, requires human permission, and explains the response.
It cannot secretly run shell commands, access unrelated resources or perform
multiple unattended operations.

## Verification

Run `cargo test --all-targets`, `cargo build --release --bins`, the
native CEF bundling script and the Linux/macOS/Windows smoke tests, then
manually verify an audio tab kept in the background and notification
permissions on unrelated origins. Remote Web Push requires a real compatible
push subscription and server-triggered notification on each supported OS.

These outcomes remain **unverified** while GitHub Actions runners cannot
execute any job steps (tracked in issue #7).
