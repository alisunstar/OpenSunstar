#!/usr/bin/env bash
set -euo pipefail

if [ $# -lt 1 ]; then
  echo "Usage: ./scripts/release-tag.sh <version>"
  echo "Example: ./scripts/release-tag.sh 0.1.1"
  exit 1
fi

raw_version="$1"
version="${raw_version#v}"
tag="v${version}"

if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Error: version must be SemVer (x.y.z), got: $raw_version"
  exit 1
fi

if ! command -v git >/dev/null 2>&1; then
  echo "Error: git is required"
  exit 1
fi

if ! command -v node >/dev/null 2>&1; then
  echo "Error: node is required"
  exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
  echo "Error: git working tree is not clean. Commit or stash changes first."
  exit 1
fi

pkg_version="$(node -e "process.stdout.write(require('./package.json').version)")"
tauri_version="$(node -e "process.stdout.write(require('./src-tauri/tauri.conf.json').version)")"

if [ "$pkg_version" != "$version" ] || [ "$tauri_version" != "$version" ]; then
  echo "Error: version mismatch."
  echo "  target:            $version"
  echo "  package.json:      $pkg_version"
  echo "  src-tauri config:  $tauri_version"
  echo "Please update both files before tagging."
  exit 1
fi

if git rev-parse "$tag" >/dev/null 2>&1; then
  echo "Error: local tag already exists: $tag"
  exit 1
fi

if git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1; then
  echo "Error: remote tag already exists: $tag"
  exit 1
fi

echo "Creating tag: $tag"
git tag -a "$tag" -m "Release $tag"

echo "Pushing branch HEAD to origin"
git push origin HEAD

echo "Pushing tag to origin"
git push origin "$tag"

echo "Done. GitHub Actions release workflow should start for $tag."
