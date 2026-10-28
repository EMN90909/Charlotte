#!/usr/bin/env bash
set -euo pipefail
file="${1:?usage: bump-changelog.sh CHANGELOG.md version}"
version="${2:?usage: bump-changelog.sh CHANGELOG.md version}"
[ -f "$file" ] || { echo "Missing $file" >&2; exit 1; }
grep -q '^## \[Unreleased\]' "$file" || { echo "Missing [Unreleased] section" >&2; exit 1; }
temp="$(mktemp)"
trap 'rm -f "$temp"' EXIT
awk -v version="$version" -v date="$(date +%Y-%m-%d)" '
  /^## \[Unreleased\]/ && !stamped {
    print
    print ""
    print "## [" version "] - " date
    stamped = 1
    next
  }
  { print }
' "$file" > "$temp"
cat "$temp" > "$file"
echo "Stamped $version into $file"
