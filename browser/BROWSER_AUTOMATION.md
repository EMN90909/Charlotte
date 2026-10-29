# Charlotte: native Chromium browser automation

## Research and integration choice

Chromium automation works by observing the loaded page, choosing a target,
issuing browser input/navigation commands, and confirming that the resulting
page state matches the goal.

The supported mechanisms relevant to Charlotte are:

1. **DOM / accessibility inspection.** Inspect a compact set of actual
   interactive elements with labels, roles, editable state, URLs and stable
   selectors. Ground the agent's next action in this live observation.
2. **Chrome DevTools Protocol (CDP).** Use Runtime.evaluate for bounded DOM
   observations and safe input-field updates, Page.navigate for navigation,
   and Input.dispatchMouseEvent for real mouse press/release events.
3. **CEF integration.** CEF exposes the CDP commands directly through
   CefBrowserHost::SendDevToolsMessage and AddDevToolsMessageObserver.
   Observers receive asynchronous method results, correlated by ID, on
   CEF's browser-process UI thread.
4. **An agent loop.** Observe -> Plan -> Obtain approval -> Act -> Verify.
   Replan from the new observation rather than blindly executing stale
   selectors on a changed page.

Source references:
- Chromium CDP documentation: https://chromedevtools.github.io/devtools-protocol/
- CEF browser-host API: https://github.com/chromiumembedded/cef/blob/master/include/cef_browser.h
- CEF observer API: https://github.com/chromiumembedded/cef/blob/master/include/cef_devtools_message_observer.h

The Rust CEF bindings already used by this repo implement the observer
registration pattern; no sidecar Chrome, Selenium server, browser extension,
external debugging port, Electron process or Node.js is needed.

## Research update: accessibility-first agents and durable approval state

The user-supplied browser automation / Strawberry-style agentic browser study
has now been incorporated into the native Charlotte implementation:

- Real Chromium Accessibility.getFullAXTree results are filtered to compact
  semantic roles/names; current DOM controls are numbered [1], [2], etc.
- The planner prefers click_index, type_index and drag_index, which Rust
  resolves to verified current selectors; indexes are never durable IDs.
- When a target selector is stale, exactly one matching role/name may be
  offered as a **new approval**, not silently clicked.
- Each run has a bounded SQLite/WAL checkpoint with a distinct run ID and
  states observing, planning, awaiting_approval, executing, verifying, done,
  paused, stopped or interrupted.
- The sidebar exposes Pause / Take Over, Inspect and re-plan saved task,
  and Discard saved task. Nothing is replayed after a crash/restart.
- The model gets a compact role/name/index snapshot, the current goal and
  the previous step result, rather than unrelated saved conversations.
- Consequential actions receive a stronger approval warning; all site
  mutations still require explicit user approval.

See [AUTOMATION_RESEARCH_INTEGRATION.md](AUTOMATION_RESEARCH_INTEGRATION.md)
for the full research comparison and outstanding capabilities. Composite
OOPIF node handles, long-term semantic memory, Skills/Routines, and global
DevTools event multiplexing remain **future work**, not completed features.

## User workflow

1. Open a normal HTTPS webpage in Charlotte.
2. Open the sidebar, select an AI provider and set **Charlotte mode** to
   **Automate this Chromium tab**.
3. Enter a concrete goal, e.g. "Find the price listed for the entry-level plan
   and open the plan details, but do not purchase anything."
4. Charlotte inspects the active page in its own Chromium process. A bounded
   snapshot is sent to the chosen provider (including up to ~5,000 characters
   of visible page text and at most 70 interactive controls).
5. The AI returns a **typed browser action**: navigate (same origin), click
   an observed element, fill a non-sensitive input, scroll or finish.
6. For every site-changing operation, the sidebar shows the real target
   site, action, selected element label, and parameters. Choose
   **Approve this browser step** or **Stop automation**.
7. After approval Charlotte executes a private CDP command on the UI thread,
   waits for Chromium to finish, observes the page again and proposes
   the next step. Maximum eight actions in one task.

The approval step is intentionally required even for ordinary clicks.
Browser pages can attach dangerous logic to seemingly harmless controls,
so labels alone are not sufficient evidence of safety.

## Architecture

- src/browser_automation.rs: typed action enum; selector, text, URL validation;
  bounded DOM observation; native Chromium mouse input; scoped navigation;
  CDP result handling; request timeout and CEF observer cleanup.
- src/app.rs: CEF UI-thread dispatcher and per-request DevTools observer.
  Validates browser ID and authorized site origin before executing requests.
- src/browser_agent.rs: model planner accepting only explicit JSON action
  variants, with a strict eight-step limit and untrusted-page instructions.
- src/ui_server.rs: mode selector, in-app approval card and user-controlled
  step-by-step execution, with the existing form token and Origin/Host checks.
- src/agent.rs + src/mcp.rs: separate approved external MCP tool workflow.

CEF Browser objects never cross Rust worker threads; only serializable
typed commands and results cross the channel.

## Safety and capability boundaries

- Website HTML/text is always untrusted input to the agent.
- No arbitrary model-generated JavaScript or unrestricted raw CDP commands.
- Observed selectors must match exactly one permitted live DOM control,
  both at planning time and immediately before execution.
- Password, hidden and file inputs are not included in inspection targets
  or typed automatically; account verification must be performed manually.
- HTTPS/loopback-only navigation with exact same-origin scope for each task.
- No switching to or controlling unrelated tabs.
- The agent cannot use browser automation to read OS files or credentials.
- CDP request identifiers are scoped, each request has a 12s timeout;
  observers are released on completion or timeout.
- DOM inspection/redacted link targets omit URL query strings and form
  input values. Visible page text may still be private: selecting browser
  mode explicitly authorizes sending that bounded observation to the
  chosen AI provider. Do not enable on sensitive websites.
- Navigation, clicks, typing and scrolling require approval; no unattended
  payments, posts, destructive submissions, login automation or CAPTCHA bypass.
- The system stops on unexpected cross-origin redirects; the user can
  begin a new task on a newly authorized site.

## Known limitations

This is a bounded, human-in-the-loop CDP agent, not unrestricted web crawling.
Currently the observation uses the main document's DOM. Cross-origin
iframes, shadow DOM, canvas-only controls, complex drag-and-drop, file uploads
and browser/system dialogs require separate target support and tests.
Some custom websites might not react to programmatic input/change events
even though native mouse clicks use CDP. Screenshots and accessibility-tree
fallback are not connected yet, which limits vision-only sites.

No reliable latency, memory, or success rate has yet been measured.
The native CEF CI jobs must first be able to run.

## Acceptance checks

Run cargo test --all-targets and cargo build --release --bins, then start
the packaged app and manually verify these cases on Windows/macOS/Linux:

- Inspect local HTTPS/loopback fixture; labels and selectors match the DOM.
- Clicking a link uses Input.dispatchMouseEvent and navigation is verified.
- Typing into a plain text field changes its value and triggers SPA events.
- Dynamic page reload between plan/approval safely rejects stale actions.
- Automatic page change, tab switch or cross-origin redirect aborts the step.
- A custom page cannot request raw CDP commands from the loopback service;
  missing Origin/token POSTs return HTTP 403.
- Stalling a CDP response times out and observer count stays bounded.
- The agent stops after eight steps; denied actions never execute.
- Existing audio tabs, ad blocker, MCP tools and notifications still behave.

Until native compilation and these tests pass, this remains a draft integration.
