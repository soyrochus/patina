#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/tag-release.sh <tag>

Creates a new release tag and pushes it to origin.
The tag must start with "v", for example:

  scripts/tag-release.sh v0.4.0

Pushing the tag triggers any GitHub Actions workflow configured for tag pushes.
USAGE
}

if [[ $# -ne 1 ]]; then
  usage >&2
  exit 64
fi

tag="$1"

if [[ "$tag" != v* || "$tag" == "v" ]]; then
  echo "error: tag must match pattern v*, for example v0.4.0" >&2
  exit 64
fi

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "error: not inside a Git worktree" >&2
  exit 1
fi

if ! git remote get-url origin >/dev/null 2>&1; then
  echo "error: Git remote 'origin' is not configured" >&2
  exit 1
fi

if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
  echo "error: local tag '$tag' already exists" >&2
  exit 1
fi

if git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1; then
  echo "error: remote tag '$tag' already exists on origin" >&2
  exit 1
fi

echo "Creating annotated tag '$tag' at $(git rev-parse --short HEAD)"
git tag -a "$tag" -m "Release $tag"

echo "Pushing tag '$tag' to origin"
git push origin "$tag"

echo "Done. GitHub Actions tag workflows should now be triggered for '$tag'."
