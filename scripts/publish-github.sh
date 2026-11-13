#!/usr/bin/env bash
# Explicit, user-run publish helper. Requires an authenticated GitHub CLI.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

repo='hamdyaea/ShellTrace'
command -v gh >/dev/null || { echo 'Install GitHub CLI (gh) first.' >&2; exit 1; }
command -v git >/dev/null || { echo 'Install git first.' >&2; exit 1; }
gh auth status >/dev/null || { echo 'Authenticate with gh auth login first.' >&2; exit 1; }

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git init -b main
fi

git add .
if ! git diff --cached --quiet; then
  git commit -m 'feat: initial ShellTrace Rust CLI and evidence reporting'
fi

if gh repo view "$repo" >/dev/null 2>&1; then
  if ! git remote get-url origin >/dev/null 2>&1; then
    git remote add origin "https://github.com/$repo.git"
  fi
  git push -u origin main
else
  gh repo create "$repo" --public \
    --description 'Rust Linux command tracing: filesystem, processes, network and sandbox evidence (GPLv3)' \
    --source=. --remote=origin --push
fi

echo "Published: https://github.com/$repo"
