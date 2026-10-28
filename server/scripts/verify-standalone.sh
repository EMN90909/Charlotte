#!/usr/bin/env bash
# Validate that server/ builds and tests with absolutely no parent-repo files.
set -euo pipefail
server_dir="$(cd "$(dirname "$0")/.." && pwd)"
temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT
mkdir -p "$temporary/isolated"
cp -R "$server_dir/." "$temporary/isolated/"
cd "$temporary/isolated"
# The self-contained module and scripts must exist in this detached copy.
test -f go.mod
test -f scripts/build-all.sh
test -f scripts/bump-version.sh
if [ "$(go env GOMOD)" != "$PWD/go.mod" ]; then
  echo "Server does not resolve its own go.mod" >&2
  exit 1
fi
make check
make build
test -f bin/charlotte-server
go test -count=1 ./...
echo "Standalone copy compiled and tests passed."
