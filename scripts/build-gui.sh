#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

# Bash may inherit an older rustup toolchain from ~/.cargo/bin even when
# Manjaro/Arch has a current Rust/Cargo under /usr/bin. Cargo 1.70 cannot
# read dependencies using Rust edition 2024.
version_supported() {
  local executable="${1:-}" output major minor
  [[ -n "$executable" && -x "$executable" ]] || return 1
  output=$("$executable" --version 2>/dev/null) || return 1
  [[ "$output" =~ [[:space:]]([0-9]+)\.([0-9]+)\.[0-9]+ ]] || return 1
  major="${BASH_REMATCH[1]}"
  minor="${BASH_REMATCH[2]}"
  (( major > 1 || (major == 1 && minor >= 85) ))
}

cargo_bin="$(command -v cargo || true)"
rustc_bin="$(command -v rustc || true)"

if ! version_supported "$cargo_bin" || ! version_supported "$rustc_bin"; then
  if version_supported "/usr/bin/cargo" && version_supported "/usr/bin/rustc"; then
    echo "Detected an outdated Rust/Cargo ahead of the system toolchain."
    echo "Using /usr/bin/cargo and /usr/bin/rustc for this build."
    export PATH="/usr/bin:$PATH"
    hash -r
  else
    echo "ShellTrace requires Rust and Cargo >= 1.85 (for edition-2024 dependencies)." >&2
    echo "Current Cargo: ${cargo_bin:-not found}" >&2
    echo "Current Rust:  ${rustc_bin:-not found}" >&2
    echo "Install/update Rust or run: PATH=\"/usr/bin:$PATH\" bash scripts/install-gui.sh" >&2
    exit 1
  fi
fi

echo "Cargo: $(command -v cargo) ($(cargo --version))"
echo "Rust:  $(command -v rustc) ($(rustc --version))"

command -v cmake >/dev/null || { echo "cmake is required" >&2; exit 1; }

cargo build --release --locked
cmake -S gui -B build/gui -DCMAKE_BUILD_TYPE=Release
cmake --build build/gui --parallel
printf '\nStart Qt6 interface:\n  ./build/gui/shelltrace-gui\n'
