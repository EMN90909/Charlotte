# Rust + CEF performance and integration contract

Charlotte uses a native Rust application hosting Chromium Embedded Framework
(CEF) Views. CEF remains a multi-process Chromium runtime, with platform-specific
shared libraries, renderer/utility helpers, sandbox resources and GPU processes.
**No Electron, Node, or WebKit runtime is introduced by this browser build.**

## Optimisations included

- **Correct thread model:** adblock-rust 0.13.3 is compiled without its
  `single-thread` feature. Its Engine can then be safely shared by CEF UI,
  IO, and background list-compiler threads. A shared Rust `RwLock` allows
  simultaneous request reads; rebuilding lists takes the write lock only
  for the final swap. A compile-time Send + Sync assertion and concurrent
  blocking-request test guard the contract.
- **Safe startup:** compile minimal offline ad-blocking rules immediately;
  load cached ABP lists and refresh them on one deduplicated background thread.
  Switching Shields OFF prevents new updates; switching it ON schedules a
  refresh. Block counters survive rule-set replacement.
- **Reduced cold-start network traffic:** saved background tabs start at
  `about:blank`; their addresses remain in the Rust tab/session registry
  and load when selected. **CEF browser views for those tabs still exist**,
  so this is not a full lazy allocation of all Chromium processes.
- **Less UI overhead:** the local loopback Axum server uses one Tokio
  current-thread reactor, while blocking I/O is dispatched to the on-demand
  blocking executor. Repeated tab selections no longer write identical
  session snapshots to SQLite; title changes do not persist URL-only session
  data.
- **Reduced page work:** cosmetic selectors are converted into a bounded,
  one-time stylesheet per page/frame. CSS handles newly created elements
  without a perpetual MutationObserver scanning the whole DOM.
- **Production Rust build:** thin LTO, one codegen unit, stripped debug info,
  and abort-on-panic for the CEF FFI boundary, without changing Chromium
  renderer security settings.

## Must not be sacrificed for speed

Do not set `--no-sandbox`, `--disable-web-security`,
`--disable-site-isolation-trials`, `--single-process`, or disable TLS
verification. Do not disable GPU acceleration by default. Those flags trade
browser security or broad page compatibility for superficial savings.
CEF resource files and helper binaries are not optional in the bundle.

## Validation matrix

Before treating Charlotte as production ready, run the following on each
supported host OS with a matching CEF distribution:

1. `cargo test --all-targets` and `cargo build --release --bins`.
2. `sh scripts/build.sh` on Linux/macOS or `scripts\\build.bat` on Windows,
   with `CEF_PATH` set, confirming all native CEF support files exist.
3. The corresponding `scripts/smoke-native*` acceptance test.
4. Interactive CEF testing: HTTPS navigation, tab creation, background
   restoration, page chat, Shields on/off, and both blocking and exception
   rules with a reproducible local test page.
5. Observe idle and 5-tab process-tree memory, cold start, CPU usage, and
   page render metrics on consistent hardware. Compare against the
   parent commit, not a generic Chromium marketing claim.

For Linux, `bash scripts/measure-native-linux.sh` prints time until the
local UI endpoint responds and approximate RSS for the CEF/Xvfb process
tree. The memory sum **double-counts shared mappings and includes Xvfb**:
it is a relative diagnostic, not private resident memory or a universal
browser memory score.

## Current verification limitation

This change has not received a successful GitHub Actions runner execution
or on-device CEF test in the current environment. Prior PR jobs failed with
zero steps, so no measured RAM/CPU reductions, successful cross-platform
builds or functional browsing can yet be certified. Keep the PR as a draft
until tests, bundles and UI smoke runs pass.
