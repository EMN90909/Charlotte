# Charlotte agentic browsing — research integration plan

Source: user-provided research, **“Browser Automation, Strawberry Browser, and Building a Reliable Agentic Browser in Rust + CEF”**, received 29 October 2026.

This document maps its proposed mechanisms into the existing Charlotte Rust + CEF architecture. Claims about other products, integration counts, comparative latency, token costs, certifications and legal compliance are **claims in the provided research, not verified benchmarks or assurances about Charlotte**. Strawberry is a comparative reference, not a dependency. Charlotte remains its own browser.

## Layer 1 — native browser transport

**Already implemented in the draft feature branch**
- Native CEF DevTools bridge through `CefBrowserHost::SendDevToolsMessage` and `CefDevToolsMessageObserver`; no extra Node/Playwright relay and no public remote debugging port.
- Bounded UI-thread request dispatch, per-tab origin guards, observer lifetime cleanup and timeouts.
- CDP methods for `Runtime.evaluate`, `Page.navigate`, `Input.dispatchMouseEvent`, `Page.captureScreenshot`, `Page.getFrameTree`, `Accessibility.getFullAXTree` and `Page.handleJavaScriptDialog`.

**Not implemented / not proven**
- Long-running `OnDevToolsEvent` subscriptions, per-browser event multiplexing, detached-target recovery, and separate OOPIF CDP sessions.
- Typed `chromiumoxide_cdp` command bindings. Do not add an entire second transport/runtime; if considered, evaluate compile-time cost and compatibility with the pinned CEF version.
- Do not claim raw CDP is inherently faster or more memory efficient without native benchmarks against a comparable workload.

## Layer 2 — observe, decide, act, verify

**Already implemented**
- Bounded open shadow-root and same-origin iframe DOM observation.
- Accessibility-tree semantic labels/roles, numbered ephemeral controls, role/name fallback that must receive *fresh human approval*, and pruned context for the LLM.
- Optional JPEG screenshot mode with explicit provider disclosure; cross-origin iframe visual interactions remain subject to approval and current-viewport checks.
- Strict JSON action schema; no arbitrary JavaScript or OS access granted to the model.
- Typed click, type, navigation, scroll, drag, dialogs; safety/approval gates and bounded action budget.

**This integration adds**
- Whitelisted accessibility states `checked`, `disabled`, `expanded`, `selected`, `required`; do not expose arbitrary text-field AX values.
- Consequential risk for typed recipient, identity, bank/payment, token and verification fields, even before a submit action. These are advisory classifications: existing per-action approval still applies.

**Later engineering**
- Fuse `DOMSnapshot.captureSnapshot` with AX while hard-limiting snapshot size, backend node IDs, text fields and privacy exposure; `DOMSnapshot` can be large or contain sensitive document material.
- Composite references `{target_id, frame_id, backend_node_id, role, accessible_name, bounding_box}` owned by Rust for live CDP nodes. Preserve target/frame isolation; never resolve a backend node ID in a different renderer.
- OOPIF attach/detach lifecycle and per-origin permission checks; do not treat visual iframe clicking as direct DOM access.
- Event-driven crash, download and JS dialog watchers using actual CEF callback signatures (verify bindings before writing); test the interaction with blocking modal loops.

## Layer 3 — durable execution, pause and memory

**Already implemented**
- SQLite WAL automation run markers with `observing`, `planning`, `awaiting_approval`, `executing`, `verifying`, `paused`/interrupted states; run IDs and step counters.
- Browser task pause/manual takeover and safe resume by **re-planning**. Previous writes are never auto-replayed on app restart.
- Charlotte does not persist screenshots, form values, untrusted page contents or action approval secrets in the run log.

**Not implemented**
- Arbitrary 50+ step autonomous execution: current capped automation remains 8 actions for user safety.
- A task/subtask dependency graph, persistent explicit approval capability that authorizes writes after restart, deterministic skill recording and replay, and an explicit opt-in preference/memory store.
- Disk-offloaded screenshot/AX trace logging. The pasted research recommends this for token savings, but Charlotte must not persist screenshots by default; a future feature would require bounded encrypted retention, explicit consent, user deletion and a private-mode block.

**Skills proposal**
- Define versioned, parameterized JSON skill templates; allow only typed browser actions, no stored raw cookies/passwords. Never automatically replay a consequential action from a previous approval.
- First implementation should be deterministic read-only extraction and navigation steps; all external writes require current-run authorization and fresh target verification.
- Require a dedicated skill permission scope (origin, allowed operations, step budget, expiration), trace provenance and clear user-controlled deletion.

## Layer 4 — sessions, user control and security

**Already implemented**
- Uses the actual active CEF tab and its authenticated browsing context. Credentials remain in host/keychain; the agent receives bounded observations rather than login secrets.
- Explicit per-step approval UI, risk designation, manual takeover, and origin-bound execution; opt-in screenshot mode.

**Not implemented**
- Parallel multi-tab agent orchestrator and trustworthy per-tab control bar with job ownership, instant pause/cancel, task isolation.
- Durable *mutating* approval after full restart: current resumption always re-inspects/re-plans; that is safer than silently preserving authorization.
- OAuth-backed multi-app workflow routing and guaranteed connector inventory; the pasted Strawberry figures describe another product and are not Charlotte functionality.
- OS dialog/file picker automation, MFA/CAPTCHA bypass, unattended financial submissions. These should **not** be automatically delegated to a model.

## Ordered release gates

1. Repair GitHub Actions provisioning (issue #7) and get actual `cargo test` + CEF builds.
2. Verify accessibility states, index resolution, risk classification, re-approval and scoped origins via Rust tests.
3. Run user-provided fixture sites in native Charlotte on Windows/macOS/Linux, including HiDPI pixel conversion and true OOPIF isolation.
4. Test pause and restart at every checkpoint (especially after an action may have executed) and ensure there is no double submission.
5. Implement and test CEF event watchdogs, target-session lifecycle and crash recovery before broadening scopes.
6. Evaluate structured skill templates and scoped replay only after permission and observability controls are robust.
7. Benchmark actual memory, latency, token usage and model success rates; no unmeasured claims.
8. Keep PR #6 in draft until native integration, regression and privacy checks pass.

## References from the supplied research

- https://chromedevtools.github.io/devtools-protocol/
- https://github.com/chromiumembedded/cef/blob/master/include/cef_browser.h
- https://github.com/chromiumembedded/cef/blob/master/include/cef_devtools_message_observer.h
- https://github.com/tauri-apps/cef-rs
- https://browser-use.com/posts/playwright-to-cdp
- https://strawberrybrowser.com/for-ai

The integration deliberately adopts relevant architecture and design principles **without copying product marketing claims** or compromising browser isolation and human approval.
