#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
command -v cmake >/dev/null || { echo "cmake is required" >&2; exit 1; }
cargo build --release --locked
cmake -S gui -B build/gui -DCMAKE_BUILD_TYPE=Release
cmake --build build/gui --parallel
printf '\nStart Qt6 interface:\n  ./build/gui/shelltrace-gui\n'
