#!/usr/bin/env bash
# User-level installer for ShellTrace Rust CLI and Qt6 GUI.
set -euo pipefail
cd "$(dirname "$0")/.."
bin_dir="${HOME}/.local/bin"
app_dir="${XDG_DATA_HOME:-${HOME}/.local/share}/applications"
desktop_file="${app_dir}/shelltrace-gui.desktop"
icon_file="${XDG_DATA_HOME:-${HOME}/.local/share}/icons/hicolor/scalable/apps/shelltrace.svg"

if [[ "${1:-}" == "--uninstall" ]]; then
  rm -f "${bin_dir}/shelltrace" "${bin_dir}/shelltrace-gui" "${desktop_file}" "${icon_file}"
  echo "Removed the user-level ShellTrace launcher and executables."
  exit 0
fi
if [[ $# -ne 0 ]]; then
  echo "Usage: bash scripts/install-gui.sh [--uninstall]" >&2
  exit 2
fi
bash scripts/build-gui.sh
mkdir -p "$bin_dir" "$app_dir"
install -m 755 target/release/shelltrace "${bin_dir}/shelltrace"
install -m 755 build/gui/shelltrace-gui "${bin_dir}/shelltrace-gui"
install -m 644 gui/shelltrace-gui.desktop "$desktop_file"
install -Dm644 assets/shelltrace-badge.svg "$icon_file"

# Desktop environment PATH is not guaranteed to include ~/.local/bin.
sed -i "s|^Exec=.*|Exec=\"${bin_dir}/shelltrace-gui\"|" "$desktop_file"
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$app_dir" >/dev/null 2>&1 || true
fi
echo "Installed ShellTrace in $bin_dir"
echo "Application entry: $desktop_file"
echo "You can now find ShellTrace in your applications menu."
