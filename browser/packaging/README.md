# Native Charlotte browser packaging

The release workflow packages CEF bundles for supported platform runners
using Rust/Cargo, the matching binary CEF SDK and bundle-cef-app.

From `browser/`, use `make build` for the binaries, `make package`
for the current operating system, or one of the platform target Makefile
rules for compiling to a configured cross toolchain.

Installers are unsigned/unnotarized until appropriate certificate signing
is configured. Do not publish a browser as production-ready until the
Linux, macOS and Windows native runtime smoke gates have passed.
