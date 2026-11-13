#!/usr/bin/env bash
# Native distribution build (run separately on each supported Debian/Ubuntu release).
set -euo pipefail
cd "$(dirname "$0")/../.."
for cmd in cargo cmake dpkg-deb dpkg-shlibdeps dpkg awk; do
  command -v "$cmd" >/dev/null || { echo "Missing required command: $cmd" >&2; exit 1; }
done
version="$(awk -F'"' '/^version = / { print $2; exit }' Cargo.toml)"
arch="$(dpkg --print-architecture)"
. /etc/os-release
flavour="${ID}${VERSION_ID//./}"
case "$flavour" in
  debian12|debian13|ubuntu2404|ubuntu2604) ;;
  *) echo "Unsupported distribution for published binaries: $flavour" >&2; exit 1 ;;
esac

cargo build --release --locked
cmake -S gui -B build/gui -DCMAKE_BUILD_TYPE=Release
cmake --build build/gui --parallel
QT_QPA_PLATFORM=offscreen ./build/gui/shelltrace-gui --self-test
QT_QPA_PLATFORM=offscreen LANG=fr_CH.UTF-8 ./build/gui/shelltrace-gui --self-test-english

root="$(mktemp -d)"
trap 'rm -rf "$root"' EXIT
package="$root/pkg"
mkdir -p "$package/DEBIAN" "$package/usr/bin" "$package/usr/share/applications"
mkdir -p "$package/usr/share/icons/hicolor/scalable/apps" "$package/usr/share/doc/shelltrace"
install -m 755 target/release/shelltrace "$package/usr/bin/shelltrace"
install -m 755 build/gui/shelltrace-gui "$package/usr/bin/shelltrace-gui"
install -m 644 gui/shelltrace-gui.desktop "$package/usr/share/applications/shelltrace-gui.desktop"
install -m 644 assets/shelltrace-badge.svg "$package/usr/share/icons/hicolor/scalable/apps/shelltrace.svg"
install -m 644 LICENSE "$package/usr/share/doc/shelltrace/copyright"
install -m 644 README.md "$package/usr/share/doc/shelltrace/README.md"

# Derive native runtime ABI dependencies from binaries, rather than hardcoding
# Ubuntu's Qt *t64 names onto Debian or vice versa.
mkdir -p "$root/build/debian"
cat > "$root/build/debian/control" <<'CTRL'
Source: shelltrace
Section: utils
Priority: optional
Maintainer: ShellTrace project <hamdyaea@users.noreply.github.com>
Standards-Version: 4.6.2

Package: shelltrace
Architecture: any
Description: Inspect the effects of Linux commands (Rust CLI and Qt6 GUI)
CTRL
pushd "$root/build" >/dev/null
deps="$(dpkg-shlibdeps -O -e"$package/usr/bin/shelltrace" -e"$package/usr/bin/shelltrace-gui" | sed -n 's/^shlibs:Depends=//p')"
popd >/dev/null
if [[ -z "$deps" ]]; then echo "dpkg-shlibdeps found no shared-library dependencies" >&2; exit 1; fi

installed="$(du -sk "$package/usr" | awk '{print $1}')"
cat > "$package/DEBIAN/control" <<CTRL
Package: shelltrace
Version: ${version}-1
Section: utils
Priority: optional
Architecture: $arch
Maintainer: ShellTrace project <hamdyaea@users.noreply.github.com>
Homepage: https://github.com/hamdyaea/ShellTrace
Installed-Size: $installed
Depends: $deps, strace, bubblewrap
Description: Inspect the effects of Linux commands
 Rust CLI and Qt6 desktop application for filesystem before/after
 snapshots, process and network syscalls, and optional Bubblewrap trials.
CTRL

mkdir -p dist
out="dist/shelltrace_${version}-1_${flavour}_${arch}.deb"
dpkg-deb --root-owner-group --build "$package" "$out"
dpkg-deb --info "$out"
echo "Built: $out"
