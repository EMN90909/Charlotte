# Charlotte's Go server is a complete module. All paths are relative to
# server/ and work when this directory is copied to its own repository.
.PHONY: help proto build build-ops build-client build-all check smoke run run-client run-ops update-ops clean test test-unit test-integration verify-standalone bump-major bump-minor bump-patch _bump tag docker

GO ?= go
BIN_DIR ?= bin
GITHUB_REPO ?= EMN90909/Charlotte
SCRIPTS := ./scripts
PROTO_DIR := proto
GEN_DIR := gen
PROTO_FILE := $(PROTO_DIR)/chat/chat.proto

help:
	@printf '%s\n' 'Charlotte server targets:' '  make build              native server -> bin/charlotte-server' '  make run                start server (CHAT_JWT_SECRET required)' '  make test               unit and integration tests' '  make verify-standalone  build/test an isolated copy of server/' '  make build-all          cross-platform release binaries -> dist/' '  make smoke              start binary and check HTTP health' '  make docker             build charlotte-server:local container' '  make proto              regenerate protobuf code (protoc + plugins)' '  make build-ops          build the deployment supervisor' '  make build-client       build the interactive chat CLI'

proto:
	@echo "Generating protobuf and gRPC bindings..."
	protoc \
		--go_out=$(GEN_DIR) --go_opt=paths=source_relative \
		--go-grpc_out=$(GEN_DIR) --go-grpc_opt=paths=source_relative \
		-I $(PROTO_DIR) \
		$(PROTO_FILE)

# No project-root assets, Node tooling, or scripts outside server/ are used.
build:
	@mkdir -p "$(BIN_DIR)"
	CGO_ENABLED=0 $(GO) build -trimpath -o "$(BIN_DIR)/charlotte-server" ./cmd/server

build-ops:
	@mkdir -p "$(BIN_DIR)"
	CGO_ENABLED=0 $(GO) build -trimpath -o "$(BIN_DIR)/charlotte-ops" ./cmd/ops

build-client:
	@mkdir -p "$(BIN_DIR)"
	CGO_ENABLED=0 $(GO) build -trimpath -o "$(BIN_DIR)/charlotte-chat" ./cmd/chat

# Retain chat-service-* release asset names for deployed ops-daemon compatibility.
build-all:
	@bash "$(SCRIPTS)/build-all.sh"

check:
	$(GO) vet ./...
	$(GO) build ./...

run:
	$(GO) run ./cmd/server

run-client:
	$(GO) run ./cmd/chat $(ARGS)

run-ops:
	$(GO) run ./cmd/ops

update-ops:
	@GITHUB_REPO="$(GITHUB_REPO)" bash deploy/update-ops.sh $(TAG)

test-unit:
	$(GO) test -race -count=1 ./internal/...

test-integration:
	$(GO) test -race -count=1 -tags integration -timeout 90s ./test/integration/...

test: test-unit test-integration

# Copies ONLY server/ into a temporary directory before building/testing.
smoke:
	@bash "$(SCRIPTS)/smoke-standalone.sh"

verify-standalone:
	@bash "$(SCRIPTS)/verify-standalone.sh"

clean:
	rm -rf "$(BIN_DIR)" dist

bump-major:
	@$(MAKE) _bump PART=major
bump-minor:
	@$(MAKE) _bump PART=minor
bump-patch:
	@$(MAKE) _bump PART=patch
_bump:
	@NEW=$$(bash "$(SCRIPTS)/bump-version.sh" VERSION "$(PART)"); \
	bash "$(SCRIPTS)/bump-changelog.sh" CHANGELOG.md "$$NEW"; \
	echo "Charlotte server bumped to $$NEW"

# Preserve the existing chat-service-v* tag convention.
# This works inside the monorepo and when server/ is its own git repo.
tag:
	@VERSION=$$(cat VERSION); \
	bash "$(SCRIPTS)/tag-release.sh" chat-service "$$VERSION" .

docker:
	docker build -t charlotte-server:local .
