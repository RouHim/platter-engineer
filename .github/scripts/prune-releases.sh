#!/usr/bin/env bash
# Keeps the newest three releases and deletes everything older including its git
# tag, so the release list stays a rolling window instead of growing by ~1.6 GB
# per weekly build.
set -euo pipefail

REPO_NAME=RouHim/platter-engineer

releases=$(gh release list \
  --repo "$REPO_NAME" \
  --limit 100 \
  --exclude-drafts \
  --json tagName,publishedAt \
  --jq 'map(select(.publishedAt != null)) | sort_by(.publishedAt) | reverse')

newest=$(jq -r '.[0].tagName // empty' <<<"$releases")
if [ -z "$newest" ]; then
  echo "No releases found, nothing to prune."
  exit 0
fi

# Releases are newest first, so everything after the first three is stale.
stale_tags=$(jq -r '.[3:][].tagName' <<<"$releases")
if [ -z "$stale_tags" ]; then
  echo "Keeping the newest 3 releases, nothing to prune."
  exit 0
fi

if grep -qxF "$newest" <<<"$stale_tags"; then
  echo "Refusing to prune: newest release ${newest} ended up in the stale set." >&2
  exit 1
fi

while IFS= read -r tag; do
  echo "Deleting release and tag: ${tag}"
  gh release delete "$tag" --repo "$REPO_NAME" --yes --cleanup-tag
done <<<"$stale_tags"
