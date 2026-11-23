# Charlotte native Windows window and settings changes

This branch addresses a release build that starts the local axum UI server but
never creates a Windows top-level window. The default non-CEF platform engine
tracked tab URLs only; it did not create a WebView2 controller or an HWND.

## Implementation

- The default Windows desktop executable opens a Tao native window containing
  Wry/WebView2 child views for the browser toolbar and rendered web page.
- The existing Rust UI server stays bound to localhost; its CSRF-protected
  HTML forms remain the command path.
- Navigation, selected URL, window size, back/forward/reload and actual page
  navigation are now connected to the desktop host.
- Double-click startup errors display a Windows error dialog.
- The local installer copies the adjacent Charlotte.exe to the current user's
  local Programs directory, creates shortcuts, and optionally launches it.
  It does not download or extract arbitrary remote files.
- Settings is now arranged into Personalization, Passwords & Security,
  AI & Agent, and Extensions Marketplace. Sidebar auto-hide is persisted
  in the existing settings database and can be disabled.
- The avatar is a dropdown with settings and browsing shortcuts.

## Build and verify on Windows 10/11

Open PowerShell in the `browser` directory:

```powershell
cargo check --all-targets
cargo test --lib
.\build.bat --dist
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify_windows_window.ps1
```

The smoke script starts the packaged 64-bit Charlotte.exe and reports PASS
only if the launched process has a non-zero native `MainWindowHandle`. A
background listener alone **does not** count as a successful GUI launch.

To test the installer, run
`dist\64-bit\windows\win8-11\charlotte-installer-x64.exe` in its original
bundle directory (alongside `Charlotte.exe`). Confirm that it offers an
installation directory, creates a desktop/Start Menu shortcut, and launches
the browser. Use `--silent` for a per-user unattended install.

## Remaining verification / known limitations

- Compilation, Windows WebView2 initialization, x86 behavior, native click
  testing, and installer shortcut creation require a Windows workstation;
  they were not exercised by the GitHub connector.
- Cargo.lock must be resolved and committed with `cargo check` after the new
  Windows-only `tao` and `wry` dependencies are fetched. Do not use
  `--locked` on the first dependency-resolution run.
- A visible WebView2 window is separate from full browser automation:
  the legacy `engine/webview2.rs` interface still includes simulated
  screenshot and JavaScript execution. Integration of the actual Wry
  controller with that automation API remains outstanding.
- Third-party extensions marketplace installation, website password
  management, and verified Windows 8/8.1 ESR compatibility are not implemented
  by this UI-only reorganization.
- The older Windows 7 release artifacts imported newer Windows API symbols;
  this branch stops packaging those unverified executables as Win7 ESR.

## Diagnosing unexpected failures

If the executable displays an error dialog, capture its message. For a
missing WebView2 runtime, install/repair the Microsoft Edge WebView2 Runtime
that matches your OS and architecture. If the window smoke test fails but the
process stays running, inspect its local port and the Windows Application
Event Log; the native window constructor will now report initialization
errors rather than silently returning success.
