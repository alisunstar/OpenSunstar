#!/usr/bin/env bash
# Sync website/ → opensunstar/opensunstar.github.io (GitHub Pages, 方式 A — org pages)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WEBSITE_DIR="$ROOT/website"

PAGES_REPO_URL="${PAGES_REPO_URL:-git@github.com:opensunstar/opensunstar.github.io.git}"
PAGES_REPO_DIR="${PAGES_REPO_DIR:-$ROOT/../opensunstar.github.io}"
PAGES_BRANCH="${PAGES_BRANCH:-main}"
COMMIT_MSG="${COMMIT_MSG:-chore: sync website from OpenSunstar monorepo}"
DRY_RUN="${DRY_RUN:-0}"
NO_PUSH="${NO_PUSH:-0}"

export GIT_CONFIG_COUNT=1
export GIT_CONFIG_KEY_0=core.quotepath
export GIT_CONFIG_VALUE_0=false

# First SSH to GitHub in WSL/Git Bash should not block on host-key prompt.
ensure_github_ssh_known_hosts() {
  if [[ "$DRY_RUN" == "1" ]]; then
    return
  fi

  mkdir -p "$HOME/.ssh"
  chmod 700 "$HOME/.ssh"

  if [[ -f "$HOME/.ssh/known_hosts" ]] && grep -q '^github.com ' "$HOME/.ssh/known_hosts" 2>/dev/null; then
    return
  fi

  echo "→ Adding github.com to SSH known_hosts (one-time)..."
  if command -v ssh-keyscan >/dev/null 2>&1; then
    ssh-keyscan -t ed25519 github.com >>"$HOME/.ssh/known_hosts" 2>/dev/null || true
  fi

  export GIT_SSH_COMMAND="${GIT_SSH_COMMAND:-ssh -o StrictHostKeyChecking=accept-new}"
}

usage() {
  cat <<'EOF'
Usage: scripts/sync-website-pages.sh

Sync website/ contents to a local clone of opensunstar.github.io and push to GitHub.

Environment:
  PAGES_REPO_URL   Default: git@github.com:opensunstar/opensunstar.github.io.git
  PAGES_REPO_DIR   Default: ../opensunstar.github.io (sibling of OpenSunstar)
  PAGES_BRANCH     Default: main
  COMMIT_MSG       Commit message when there are changes
  DRY_RUN=1        Show actions only, do not copy/commit/push
  NO_PUSH=1        Commit locally but do not push

First-time setup (once):
  1. Create org: github.com/account/organizations/new → opensunstar
  2. Transfer repo: opensunstar.github.io to the org
  3. GitHub → Settings → Pages → Deploy from branch / main / root
  4. Run: pnpm website:publish

EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ ! -f "$WEBSITE_DIR/index.html" ]]; then
  echo "error: missing $WEBSITE_DIR/index.html" >&2
  exit 1
fi

run() {
  if [[ "$DRY_RUN" == "1" ]]; then
    echo "[dry-run] $*"
  else
    "$@"
  fi
}

ensure_pages_repo() {
  if [[ -d "$PAGES_REPO_DIR/.git" ]]; then
    echo "→ Updating pages repo: $PAGES_REPO_DIR"
    run git -C "$PAGES_REPO_DIR" fetch origin "$PAGES_BRANCH" 2>/dev/null || true
    run git -C "$PAGES_REPO_DIR" checkout "$PAGES_BRANCH" 2>/dev/null || run git -C "$PAGES_REPO_DIR" checkout -B "$PAGES_BRANCH"
    if git -C "$PAGES_REPO_DIR" rev-parse "origin/$PAGES_BRANCH" >/dev/null 2>&1; then
      run git -C "$PAGES_REPO_DIR" pull --ff-only origin "$PAGES_BRANCH"
    fi
    return
  fi

  echo "→ Cloning $PAGES_REPO_URL"
  echo "  into $PAGES_REPO_DIR"
  if [[ "$DRY_RUN" == "1" ]]; then
    echo "[dry-run] git clone -b $PAGES_BRANCH $PAGES_REPO_URL $PAGES_REPO_DIR"
    return
  fi

  if ! git clone -b "$PAGES_BRANCH" "$PAGES_REPO_URL" "$PAGES_REPO_DIR" 2>/dev/null; then
    echo "→ Branch $PAGES_BRANCH not found, cloning default branch..."
    if ! git clone "$PAGES_REPO_URL" "$PAGES_REPO_DIR"; then
      echo "" >&2
      echo "error: cannot clone $PAGES_REPO_URL" >&2
      echo "  • Create the repo on GitHub first (opensunstar/opensunstar.github.io)" >&2
      echo "  • On Windows, use Git Bash keys — run: pnpm website:publish (not WSL bash)" >&2
      echo "  • Or set HTTPS: PAGES_REPO_URL=https://github.com/opensunstar/opensunstar.github.io.git" >&2
      exit 128
    fi
    git -C "$PAGES_REPO_DIR" checkout -B "$PAGES_BRANCH"
  fi
}

sync_website_files() {
  echo "→ Syncing $WEBSITE_DIR → $PAGES_REPO_DIR"
  if [[ "$DRY_RUN" == "1" ]]; then
    echo "[dry-run] mirror website/ (exclude README.md), preserve .git CNAME .nojekyll"
    return
  fi

  shopt -s dotglob nullglob
  for entry in "$PAGES_REPO_DIR"/* "$PAGES_REPO_DIR"/.[!.]* "$PAGES_REPO_DIR"/..?*; do
    [[ -e "$entry" ]] || continue
    base="$(basename "$entry")"
    case "$base" in
      .git | CNAME | .nojekyll) continue ;;
    esac
    rm -rf "$entry"
  done

  for item in "$WEBSITE_DIR"/*; do
    [[ -e "$item" ]] || continue
    base="$(basename "$item")"
    [[ "$base" == "README.md" ]] && continue
    cp -a "$item" "$PAGES_REPO_DIR/"
  done
}

commit_and_push() {
  if [[ "$DRY_RUN" == "1" ]]; then
    echo "[dry-run] git add -A && git commit && git push"
    return
  fi

  if [[ ! -d "$PAGES_REPO_DIR/.git" ]]; then
    echo "error: pages repo not initialized (dry-run?)" >&2
    exit 1
  fi

  if [[ -z "$(git -C "$PAGES_REPO_DIR" status --porcelain)" ]]; then
    echo "✓ Pages repo already up to date — nothing to commit."
    return
  fi

  git -C "$PAGES_REPO_DIR" add -A
  git -C "$PAGES_REPO_DIR" status --short
  git -C "$PAGES_REPO_DIR" commit -m "$COMMIT_MSG"

  if [[ "$NO_PUSH" == "1" ]]; then
    echo "✓ Committed locally (NO_PUSH=1, skipped push)."
    return
  fi

  git -C "$PAGES_REPO_DIR" push origin "$PAGES_BRANCH"
  echo ""
  echo "✓ Published. GitHub Pages may take 1–2 minutes:"
  echo "  https://opensunstar.github.io/"
}

echo "OpenSunstar website → GitHub Pages sync"
echo "  source:  $WEBSITE_DIR"
echo "  target:  $PAGES_REPO_DIR"
echo "  remote:  $PAGES_REPO_URL"
echo "  branch:  $PAGES_BRANCH"
echo ""

ensure_github_ssh_known_hosts
ensure_pages_repo
sync_website_files
commit_and_push
