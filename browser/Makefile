# Charlotte browser: native Rust + CEF. No npm, Electron, Forge or Vite.
CARGO ?= cargo
RUSTUP ?= rustup
.PHONY: all build dev check test fmt lint measure clean package package-windows release  linux-x64 linux-arm64 windows-x64 windows-arm64 macos-x64 macos-arm64  freebsd-x64 bump-major bump-minor bump-patch tag help
all: build
help:
	@echo 'Charlotte native Rust+CEF targets:'
	@echo ' build dev check test fmt lint measure package package-windows clean'
	@echo ' linux-x64 linux-arm64 windows-x64 windows-arm64 macos-x64 macos-arm64 freebsd-x64'
	@echo ' Cross-target builds require matching linker, CEF SDK and platform toolchain.'
build:
	$(CARGO) build --release --bins
dev:
	$(CARGO) run --bin Charlotte
check:
	$(CARGO) check --all-targets
test:
	$(CARGO) test --all-targets
fmt:
	$(CARGO) fmt --all
lint:
	$(CARGO) clippy --all-targets -- -D warnings
measure:
	bash scripts/measure-native-linux.sh
clean:
	$(CARGO) clean
package:
	sh scripts/build.sh
package-windows:
	powershell -ExecutionPolicy Bypass -File scripts/package-windows.ps1
release: package
linux-x64:
	$(RUSTUP) target add x86_64-unknown-linux-gnu
	$(CARGO) build --release --bins --target x86_64-unknown-linux-gnu
linux-arm64:
	$(RUSTUP) target add aarch64-unknown-linux-gnu
	$(CARGO) build --release --bins --target aarch64-unknown-linux-gnu
windows-x64:
	$(RUSTUP) target add x86_64-pc-windows-msvc
	$(CARGO) build --release --bins --target x86_64-pc-windows-msvc
windows-arm64:
	$(RUSTUP) target add aarch64-pc-windows-msvc
	$(CARGO) build --release --bins --target aarch64-pc-windows-msvc
macos-x64:
	$(RUSTUP) target add x86_64-apple-darwin
	$(CARGO) build --release --bins --target x86_64-apple-darwin
macos-arm64:
	$(RUSTUP) target add aarch64-apple-darwin
	$(CARGO) build --release --bins --target aarch64-apple-darwin
freebsd-x64:
	$(RUSTUP) target add x86_64-unknown-freebsd
	$(CARGO) build --release --bins --target x86_64-unknown-freebsd
bump-major:
	python3 scripts/bump-version.py major
bump-minor:
	python3 scripts/bump-version.py minor
bump-patch:
	python3 scripts/bump-version.py patch
tag:
	@VERSION=$$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1); 	../scripts/tag-release.sh browser "$$VERSION" .
