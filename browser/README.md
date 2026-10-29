# Charlotte Native (Rust + CEF)

This directory is Charlotte's native desktop application. It replaces the Electron host with Rust and Chromium Embedded Framework (CEF). No Node or Electron runtime is required at launch.

| Platform | Status | Output |
| --- | --- | --- |
| Windows x64 / ARM64 | supported | installer or portable bundle |
| macOS x64 / Apple Silicon | supported | `.app` bundle |
| Linux x64 / ARM64 | supported | portable bundle |
| FreeBSD | experimental | source build against system CEF |
| OpenBSD | blocked upstream | no maintained CEF port |

Use `scripts/build.sh` on Unix-like systems or `scripts/build.bat` on Windows. `cef-dll-sys` downloads the matching CEF runtime on the first build unless `CEF_PATH` is set.
