# Building the Charlotte browser (native Rust + CEF)

Everything in this guide lives under `browser/`. The build launchers do **not**
compile the website, external API, or any other project in the repository.
Use a machine running the target OS. Cross-OS release packaging requires
the matching native compiler, CEF runtime and linker; these scripts do **not**
pretend to cross-compile CEF.

## Windows — `browser/build.bat`

Install the stable Rust toolchain (with `cargo` in PATH), Visual Studio
2022 C++ build tools/Windows SDK and a CEF binary distribution matching the
version pinned in `browser/Cargo.toml`. Also install `bundle-cef-app`
from the matching `cef-rs` build tooling and place it on PATH. Export
`CEF_PATH` to the extracted CEF distribution directory:

```bat
cd browser
set "CEF_PATH=C:\path\to\cef_binary_distribution"
build.bat
```

The default command builds **release binaries** and runs the existing native
Windows CEF bundler. Run the **bundled** launcher produced under
`browser/target/bundle` rather than the raw Rust executable: Windows
CEF sandbox builds rely on the packaged bootstrap.

Optional build modes:

```bat
build.bat --check
build.bat --test
build.bat --compile-only
build.bat --verify
build.bat --help
```

`--verify` checks all targets, runs Rust tests, builds release binaries and
runs CEF bundling. The actual Windows smoke harness, once the bundle runs,
is `powershell -ExecutionPolicy Bypass -File scripts/smoke-native.ps1`.
The optional Windows installer requires Inno Setup; see
`scripts/package-windows.ps1`.

## Linux / macOS — `browser/build.sh`

Install stable Rust (Cargo), a compatible C/C++ linker and the matching native
CEF binary distribution. Install `bundle-cef-app` from the matching
`cef-rs` tooling and add it to PATH.

```sh
cd browser
export CEF_PATH="/path/to/cef_binary_distribution"
sh build.sh
```

The default command compiles release binaries and builds a **native CEF
bundle** under `browser/target/bundle`, using the existing
`browser/scripts/build.sh`.

Other modes:

```sh
sh build.sh --check
sh build.sh --test
sh build.sh --compile-only
sh build.sh --verify
sh build.sh --help
```

Run Linux native smoke via `bash scripts/smoke-native.sh` (requires Xvfb
and curl) or macOS via `bash scripts/smoke-native-macos.sh` (requires
native GUI/CEF). These scripts use disposable test profiles.

## What the scripts do — and do not do

| Mode | Cargo check | Cargo tests | Cargo release build | CEF bundle |
|---|---|---|---|---|
| No arguments / `--bundle` | — | — | Yes | Yes |
| `--verify` | Yes | Yes | Yes | Yes |
| `--check` | Yes | — | — | — |
| `--test` | — | Yes | — | — |
| `--compile-only` | — | — | Yes | — |

The default mode fails with an informative error if `CEF_PATH` or
`bundle-cef-app` is unavailable. `--compile-only` is for source
development; it **does not create a runnable standalone Chromium browser**.
No binary or CEF SDK is committed to GitHub, and the scripts do not
download an SDK, install dependencies or bypass the CEF sandbox.

This repository currently pins `cef = 156.2.0+156.0.14`. A matching CEF
runtime is required. The source may also require platform-specific CEF build
tooling: an SDK version string is not proof of binary compatibility.

## Verification status

These build entry points and their documentation are committed code, not
evidence of successful compilation. GitHub Actions workflows previously
failed while provisioning runners, with **zero steps executed**. They must be
restored, and native release builds, Rust tests and CEF smoke tests must pass
before publishing Charlotte as a working commercial browser.

Additional runtime constraints (OAuth vendor support, desktop OS keychain
integration, site-isolated CDP, model provider compatibility and end-to-end
agent workflows) remain described in `AUTO_ORCHESTRATION.md` and
`MCP_CATALOG.md`.
