#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

export GIT_CONFIG_COUNT=1
export GIT_CONFIG_KEY_0=core.quotepath
export GIT_CONFIG_VALUE_0=false

# docs/ root *.md except kanban.md
mapfile -t INTERNAL < <(git ls-files docs/ | grep -E '^docs/[^/]+\.md$' | grep -v '^docs/kanban\.md$')

echo "Internal docs to untrack (${#INTERNAL[@]}):"
printf '  %s\n' "${INTERNAL[@]}"

if ((${#INTERNAL[@]} == 0)); then
  echo "Nothing to remove from index."
  exit 0
fi

git rm --cached -- "${INTERNAL[@]}"
echo "Removed from index (local files kept)."
