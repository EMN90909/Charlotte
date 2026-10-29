# Charlotte browser automation — research integration

Based on the user-supplied "Browser Automation, Strawberry Browser, and Building a Reliable Agentic Browser in Rust + CEF" (29 October 2026). External product numbers, comparisons and compliance statements in that document are background claims, not Charlotte benchmarks or verified third-party endorsements.

## Architecture retained

Charlotte remains an independent native Rust + CEF browser. Its browser automation uses the currently embedded Chromium through CEF's in-process DevTools interface; no Electron or Node relay, WebDriver service, ChromeDriver or public remote debugging port is introduced.

The research compares W3C WebDriver Classic, CDP, WebDriver BiDi, and extension/system APIs. Charlotte's implementation uses in-process Chromium CDP, not a cross-browser WebDriver or OS accessibility driver.

## Research-to-implementation map

| Source idea | Charlotte implementation | State |
| --- | --- | --- |
| Native CDP commands and callbacks | app.rs / browser_automation.rs; bounded method allowlist, per-request observer, timeout and detach error | Code added, native verification pending |
| Observe-Decide-Act-Verify | browser_agent.rs / ui_server.rs, eight-step task bound and explicit site-action approvals | Code added, native verification pending |
| Accessibility first | automation_semantics.rs: Chromium Accessibility.getFullAXTree, privacy-pruned semantic summary | Code added |
| Stable numbered controls | Ephemeral [1], [2] role/name handles; click_index, type_index, drag_index resolved to real current selectors | Code added |
| Semantic stale-selector recovery | Unique role/name re-query creates a new proposal and requires fresh approval; no silent rerouting | Code added |
| Risk classifier | Read only / Website interaction / Consequential action warning; explicit approval remains required for any site mutation | Code added, classifier is heuristic |
| Durable execution | SQLite WAL checkpoints store run ID, origin, goal, provider, step, status and summary | Code added |
| Human takeover | Signed Pause / Take Over button, checked before further planning and before native input dispatch | Code added |
| Session safety | Same active top-level tab, same authorised origin, credentials in OS keychain | Existing architecture |
| Compact context | Semantic index and accessible names sent to LLM instead of full CSS selectors | Code added |
| Visual automation | Opt-in bounded screenshot, JPEG dimensions validated, no persistent screenshots | Existing draft integration |
| Shadow DOM / same-origin frames | Open shadow-root and accessible iframe selectors | Existing draft integration |
| Frame watchdog | Per-request CDP timeout and agent-detached error | Partial: no global event bus |
| Downloads | Existing terminal download records in SQLite | Partial: no link to an agent run |

## Durable checkpoint rules

The local database retains at most 25 recent automation records. Checkpoint states include observing, planning, awaiting_approval, executing, verifying, interrupted, paused, stopped and done. It intentionally never stores the actual pending action JSON, typed field values, cookies, screenshots, credentials, or approval tokens.

After an app crash or restart, the user may choose "Inspect and re-plan saved task" on the original site. Charlotte must inspect the current page and request fresh approval before any action. It cannot assume a previously attempted click was rolled back or safe to repeat. Users can discard saved tasks.

The source recommends persisting complete pending tool calls. Charlotte uses a more conservative checkpoint because the real effects of a site mutation can be uncertain after a crash.

## Privacy and approval

Only a compact role/name/index representation and limited page excerpt are sent to an AI provider in ordinary browser mode. Visual mode explicitly adds a screenshot, which may contain sensitive text and cross-origin iframe pixels. Password/file inputs and raw form values are excluded from the DOM observation. User goals are stored locally in SQLite, so avoid putting sensitive personal secrets in goals.

Every browser mutation requires approval. "Consequential" is a warning tier, not a reliable proof of whether a site action is safe. When an element selector becomes stale, role/name matching is permitted only when exactly one candidate exists, and that candidate is shown for a new user approval.

Pausing blocks subsequent planned or undispatched actions, but a Chromium action already dispatched may have effects and cannot be undone automatically.

## Research recommendations not yet completed

- Composite node handles with target_id, frame_id and backend_node_id, DOMSnapshot paint-order verification, DOM.getContentQuads, and isolated OOPIF Target.setAutoAttach sessions
- Persistent CEF DevTools event multiplexing and complete renderer crash, JS dialog and download event correlations
- Multiple autonomous logged-in tabs operating in parallel, each with explicit per-tab observability and permissions
- Skill recording, parameterised deterministic replay, automatic selector-healing workflows and slash-command skill invocations
- Recurring routines, trigger/webhook engine, and unattended scheduled jobs
- Disk-offloaded full observation logs, opt-in activity recording, local vector semantic memory, and optional cloud sync
- Broad OAuth connector catalogs and native integrations; no third-party provider integration counts are claimed
- Compliance certifications or measured runtime and memory performance

The user-supplied research recommends durable full screenshots and memory. Charlotte intentionally does not collect ongoing screen recordings or persist screenshot images by default. Such a capability would require explicit opt-in and strong retention/deletion controls.

## Validation needed before merge

1. Fix GitHub runner provisioning (issue #7), then compile and test the exact pinned Rust CEF build.
2. Native unit and E2E tests for AX index roles, selector grounding, shadow DOM and iframe controls, DOM churn, visual click, drag/drop and JS dialogs.
3. Simulate app restart during pending approval and after an unknown action outcome; confirm re-observation and fresh approval rather than blind replay.
4. Simulate Pause during model processing and immediately before an approved input dispatch; verify old runs cannot acquire a newer run's authority.
5. Validate that credentials, typed field values and screenshots are not written to the automation checkpoint table.
6. Measure actual CPU/memory/task latency and task success against real Chromium. No such benchmark is claimed yet.

The PR must remain a draft until native builds and acceptance tests pass.

## References

- https://chromedevtools.github.io/devtools-protocol/tot/Accessibility/
- https://chromedevtools.github.io/devtools-protocol/tot/DOMSnapshot/
- https://chromedevtools.github.io/devtools-protocol/tot/Target/
- https://github.com/chromiumembedded/cef/blob/master/include/cef_browser.h
- https://github.com/chromiumembedded/cef/blob/master/include/cef_devtools_message_observer.h
