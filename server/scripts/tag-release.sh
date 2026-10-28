#!/usr/bin/env bash
set -euo pipefail
prefix="${1:?tag-release.sh prefix version [dir]}"
version="${2:?tag-release.sh prefix version [dir]}"
check_dir="${3:-.}"
tag="${prefix}-v${version}"
git rev-parse --is-inside-work-tree >/dev/null 2>&1 || { echo "Requires a Git checkout" >&2; exit 1; }
if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
  echo "Tag already exists: $tag" >&2; exit 1
fi
# Explicitly prevent a version tag that points at uncommitted sources.
if [ -n "$(git status --porcelain -- "$check_dir")" ]; then
  echo "Commit changes under $check_dir before creating $tag" >&2
  exit 1
fi
git tag -a "$tag" -m "$prefix $version"
echo "Created $tag; publish with: git push origin $tag"
