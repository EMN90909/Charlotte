# Charlotte Advanced Chromium Automation

## Architecture

Charlotte operates through its own embedded Chromium CEF process using the
Chrome DevTools Protocol (CDP). It does not launch Selenium, ChromeDriver,
Playwright, a remote debugging port or a second Chromium installation.

The new `src/advanced_automation.rs` complements the original typed Rust
automation driver, and `src/services.rs` supplies opt-in multimodal requests
to the selected AI provider.

### Screenshot-based visual navigation

- **Visual automation (shares a screenshot)** is a separate, explicit mode
  alongside chat/MCP and DOM-only browser automation.
- Each step uses `Page.captureScreenshot` with **viewport-only JPEG at 48%
  quality**, bounded to approximately 1.2 MB compressed.
- JPEG magic bytes and dimensions are validated in Rust before a screenshot
  can leave the browser process. Images remain ephemeral and are not added to
  the session DB or logged.
- The screenshot is accompanied by DOM data, viewport CSS dimensions and
  image pixel dimensions. Visual coordinates must use **CSS viewport pixels**,
  not raw image pixels (which differ on Retina / HiDPI displays).
- The agent may propose `click_at` on a visible image region. **Only visual
  mode** permits these coordinate actions.
- The UI displays the action and awaits **Approve this browser step**.
  Before clicking, Charlotte checks the active browser, origin, URL,
  viewport and a fresh screenshot fingerprint. Changing pixels cancel the
  pending click; the agent must re-plan.

**Privacy**: The complete visible screenshot (which can contain private data
or other websites' iframe contents) is transmitted to your selected external
AI provider when you choose visual mode. Avoid visual mode on sensitive sites.
The existing plain browser mode does **not** transmit screenshots.

Visual API envelopes are implemented for OpenAI, Claude, Gemini and a
locally installed Ollama vision model (default: `llava`). Model/provider
availability and compatibility require live testing.

### Open shadow DOM

The bounded DOM observer traverses main-document elements and **open**
`ShadowRoot` objects. It produces path selectors containing ` >>> ` to
denote a shadow boundary, e.g.:

    html > body > article-card:nth-of-type(1) >>> button:nth-of-type(1)

Selectors are resolved fresh at action time. The agent cannot invent
selectors: each must match exactly one permitted control in the live
observation. **Closed shadow roots** cannot be inspected; they may be
interacted with visually if visible.

### Iframes

- **Same-origin iframes**: The observer can inspect the iframe's
  `contentDocument` and creates scoped selectors using ` ::frame `.
  Coordinates are converted into the top-level Chromium viewport.
- **Cross-origin iframe/OOPIF**: Charlotte does not try to circumvent same
  origin policy. It inventories frame locations with `Page.getFrameTree`,
  reports iframe bounding rectangles where available, and may interact
  with their visible pixels using opt-in visual mode and approved coordinate
  clicks. It does **not** have DOM/selector access inside a cross-origin frame.
- An agent is scoped to one top-level website. It cannot programmatically
  navigate the top-level page to a second origin in one authorization.
- Dynamic cross-origin iframes may still require actual CDP session
  attachment and separate target permission handling to support richer
  interaction. This remains **not implemented**.

### Drag-and-drop

The `drag` action takes two observed selectors. The source must be reported
as `draggable`. It uses `Input.dispatchMouseEvent`:
move → press → bounded interpolated held-button moves → release.
Even when an intermediate move fails, the agent attempts to release the
mouse button. User approval is required before a drag.

This supports common mouse-driven draggable interfaces, but full HTML5
drag-data operations, operating-system file drags, cross-process drags and
complex custom libraries remain **unverified**. The CDP
`Input.dispatchDragEvent` facility is documented as experimental.

### Native dialogs

The existing CEF browser continues to own its native file pickers, platform
permission UI and download dialogs; Charlotte does not bypass them.

For JavaScript `alert()`, `confirm()`, and `prompt()`:
- The sidebar exposes user-directed **Accept open dialog** and
  **Dismiss open dialog** controls, with optional limited prompt text.
- These issue `Page.handleJavaScriptDialog` through the same authorized
  CEF UI-thread dispatcher.
- A modeled `dialog` action must also be approved before execution.
- A blocking dialog may prevent normal DOM inspection. Dialog handling
  avoids a DOM query and instead uses the existing CEF tab/origin check.
- Runtime behavior with platform-native modal event loops needs testing;
  a native modal may make the sidebar inaccessible until the user manually
  dismisses the dialog.

Automating arbitrary OS file chooser paths, password prompts, native
system permission dialogs, payment authentication and CAPTCHA challenges
is **not supported**.

## Capacity and safety

- No arbitrary model-generated JavaScript, CDP methods or shell commands.
- Eight action limit per user request; user approval for every mutation.
- No unattended credential entry or checkout completion.
- Maximum 70 observed controls, 30 frame/shadow roots and 24 frame-tree
  entries, each time bounded. Cross-origin frame addresses have queries
  and credential information removed.
- One viewport screenshot per visual planning step, with size limits;
  screenshot pixels are not persisted.
- CEF DevTools observer results are limited to 2.1 MB, concurrent requests
  to 16, and each request to a 12-second timeout.
- Stale screenshot comparisons, mismatched origin and changed URLs halt
  actions instead of guessing at coordinates.

## Local fixture for native browser verification

Two fixture pages are included in `browser/tests/fixtures/`:

- `advanced-automation.html` — ordinary controls, open shadow-root
  component, same-origin srcdoc iframe, HTML5 drag/drop, confirm and prompt.
- `cross-origin.html` — visually distinct button inside a separate-origin
  iframe.

To serve them locally, run in a terminal from `browser/tests/fixtures`:

```sh
python3 -m http.server 8877 --bind 127.0.0.1
```

Then in a second terminal from the same directory run:

```sh
python3 -m http.server 8878 --bind localhost
```

Navigate **Charlotte** to
`http://127.0.0.1:8877/advanced-automation.html`. The second iframe
will load from `http://localhost:8878/cross-origin.html` and be isolated
from the parent origin. Chromium may or may not allocate a separate
renderer process for it; verify actual OOPIF identity before claiming
out-of-process frame tests passed.

Suggested tasks:

- "Click the Shadow button once and report its new count."
- "Click the button in the same-origin iframe and report its status."
- "In visual mode click the button shown inside the cross-origin frame,
  with my approval." (No cross-origin DOM read is permitted.)
- "Drag the draggable card into the Drop target and verify the result."
- "Open the confirm dialog and let me explicitly accept or dismiss it."

File names are static test assets. They do **not** ship as background
browser automation services or require Node.js at runtime.

## Native verification before release

The following remain acceptance tests rather than passing test claims:

1. `cargo test --all-targets` and `cargo build --release --bins`
   against the exact pinned CEF 156 bindings.
2. Windows, macOS/HiDPI, Linux native screenshot captures; JPEG dimensions,
   quality and expected CSS coordinate scaling.
3. A fixture with open custom-element shadow roots and nested same-origin
   iframe buttons/fields; targeted selectors resolve correctly.
4. A fixture with a genuine OOPIF; iframe is inventoried and visual click
   works only with approval, while direct DOM access remains blocked.
5. HTML5 and custom drag/drop fixtures; drag does not leave the pointer held.
6. JavaScript alert, confirm, prompt, page-close; user manually resolves
   native file chooser without exposing paths to the model.
7. Edited/reflowing/animated page between observation and approval stops
   stale screenshot clicks rather than activating another element.
8. Test visual screenshot data privacy and model provider error handling;
   ensure tab sleeping, Shields, site notifications and MCP regressions pass.
9. Benchmark additional page-inspection memory and latency overhead; no
   measured reduction or success rate is claimed.

The GitHub runner provisioning failures (Charlotte issue #7) remain a
blocker to certifying these changes. Keep PR #6 in draft until tested.

## Technical references

- https://chromedevtools.github.io/devtools-protocol/tot/Page/
- https://chromedevtools.github.io/devtools-protocol/tot/DOMSnapshot/
- https://chromedevtools.github.io/devtools-protocol/tot/Target/
- https://chromedevtools.github.io/devtools-protocol/tot/Input/
- https://github.com/chromiumembedded/cef/blob/master/include/cef_devtools_message_observer.h
