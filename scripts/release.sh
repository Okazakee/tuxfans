#!/usr/bin/env bash
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "Usage: $0 <version>"
  echo "  e.g. $0 0.3.0"
  exit 1
fi

VERSION="$1"
TAG="v$VERSION"

# Check working tree is clean
if ! git diff --quiet; then
  echo "Working tree has uncommitted changes. Commit or stash first."
  exit 1
fi

# Bump version in Cargo.toml
sed -i "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml

# Regenerate Cargo.lock
cargo build 2>/dev/null

# Commit
git add Cargo.toml Cargo.lock
git commit -m "Bump version to $VERSION"

# Tag and push
git tag "$TAG"
git push
git push origin "$TAG"

echo "Released $TAG"
echo "→ https://github.com/Okazakee/tuxfans/actions"
