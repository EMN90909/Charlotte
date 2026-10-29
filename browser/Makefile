# Charlotte's default browser build is the native Rust + CEF application.
.PHONY: all build test check package tag bump-major bump-minor bump-patch _bump legacy-electron
CARGO ?= cargo
all: build
build:
	cd native && $(CARGO) build --release --bins
test:
	cd native && $(CARGO) test --lib
check:
	cd native && $(CARGO) check --all-targets
package:
	cd native && sh scripts/build.sh
legacy-electron:
	npm run start
bump-major:
	python3 native/scripts/bump-version.py major
bump-minor:
	python3 native/scripts/bump-version.py minor
bump-patch:
	python3 native/scripts/bump-version.py patch
tag:
	@VERSION=$$(sed -n 's/^version = "\([^"]*\)"/\1/p' native/Cargo.toml | head -1); \
	../scripts/tag-release.sh browser "$$VERSION" .
