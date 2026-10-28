#!/usr/bin/env bash
set -euo pipefail
file="${1:?usage: bump-version.sh VERSION major|minor|patch}"
part="${2:?usage: bump-version.sh VERSION major|minor|patch}"
[ -f "$file" ] || { echo "Version file missing: $file" >&2; exit 1; }
current="$(tr -d '[:space:]' < "$file")"
if ! [[ "$current" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Invalid semver: $current" >&2; exit 1
fi
IFS='.' read -r major minor patch <<< "$current"
case "$part" in
  major) major=$((major + 1)); minor=0; patch=0 ;;
  minor) minor=$((minor + 1)); patch=0 ;;
  patch) patch=$((patch + 1)) ;;
  *) echo "Invalid bump part: $part" >&2; exit 1 ;;
esac
version="$major.$minor.$patch"
printf '%s\n' "$version" > "$file"
printf '%s\n' "$version"
