.PHONY: build build-ops build-client test check run run-ops run-client smoke verify-standalone build-all docker clean update-ops
CARGO ?= cargo
BIN_DIR ?= bin
all: build
build:
	$(CARGO) build --release --bin charlotte-server
	mkdir -p $(BIN_DIR)
	cp target/release/charlotte-server $(BIN_DIR)/charlotte-server
build-ops:
	$(CARGO) build --release --bin charlotte-ops --bin charlotte-deploytoken
	mkdir -p $(BIN_DIR)
	cp target/release/charlotte-ops $(BIN_DIR)/charlotte-ops
	cp target/release/charlotte-deploytoken $(BIN_DIR)/charlotte-deploytoken
build-client:
	$(CARGO) build --release --bin charlotte-chat
	mkdir -p $(BIN_DIR)
	cp target/release/charlotte-chat $(BIN_DIR)/charlotte-chat
check:
	$(CARGO) check --all-targets
test:
	$(CARGO) test --all-targets
run:
	$(CARGO) run --bin charlotte-server
run-ops:
	$(CARGO) run --bin charlotte-ops
run-client:
	$(CARGO) run --bin charlotte-chat -- $(ARGS)
build-all:
	bash scripts/build-all.sh
smoke:
	bash scripts/smoke-standalone.sh
verify-standalone:
	bash scripts/verify-standalone.sh
docker:
	docker build -t charlotte-server:local .
clean:
	$(CARGO) clean
	rm -rf $(BIN_DIR) dist
update-ops:
	bash deploy/update-ops.sh $(TAG)
bump-major:
	bash scripts/bump-version.sh VERSION major
bump-minor:
	bash scripts/bump-version.sh VERSION minor
bump-patch:
	bash scripts/bump-version.sh VERSION patch
tag:
	bash scripts/tag-release.sh chat-service "$$(cat VERSION)" .
