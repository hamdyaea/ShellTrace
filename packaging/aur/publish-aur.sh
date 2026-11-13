#!/usr/bin/env bash
# Push only the AUR metadata, never the upstream project tree.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
command -v makepkg >/dev/null || { echo 'makepkg is required (Arch Linux / Manjaro).' >&2; exit 1; }
command -v ssh >/dev/null || { echo 'OpenSSH client required.' >&2; exit 1; }
command -v git >/dev/null || { echo 'git required.' >&2; exit 1; }
echo 'Publishing with the SSH key registered in your AUR account.'
# Some SSH endpoints exit non-zero after displaying the greeting.
# The git clone below is the actual authentication/authorization check.
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
git -c init.defaultBranch=master clone ssh://aur@aur.archlinux.org/shelltrace-git.git "$tmp/shelltrace-git"
cd "$tmp/shelltrace-git"
cp "$root/packaging/aur/PKGBUILD" PKGBUILD
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
if git diff --cached --quiet; then
  echo 'No metadata changes to publish.'
  exit 0
fi
git commit -m "Update ShellTrace AUR source package"
git push origin HEAD:master
echo 'Published: https://aur.archlinux.org/packages/shelltrace-git'
