# ShellTrace v0.3.0 — Linux distribution release

This release packages **the Rust tracing engine and Qt6 graphical application**.

## What's new

- Selected shield badge branding for the app window and desktop launcher.
- Native Qt6 theme switcher: **Dark** (default), **Light**, **System**, with persisted preference.
- Debian/Ubuntu native `.deb` packages for **Debian 12**, **Debian 13**, **Ubuntu 24.04**, **Ubuntu 26.04** (amd64).
- Arch/Manjaro **shelltrace-git** source package under `packaging/aur`, including a script for publishing to AUR over SSH.
- Both CLI and GUI included, with application desktop icon, documentation and GPLv3 license.

## Downloads

Visit https://github.com/hamdyaea/ShellTrace/releases/tag/v0.3.0

Choose exactly the file for your installed distribution:

| Distribution | Release asset |
| --- | --- |
| Debian 12 | `shelltrace_0.3.0-1_debian12_amd64.deb` |
| Debian 13 | `shelltrace_0.3.0-1_debian13_amd64.deb` |
| Ubuntu 24.04 | `shelltrace_0.3.0-1_ubuntu2404_amd64.deb` |
| Ubuntu 26.04 | `shelltrace_0.3.0-1_ubuntu2604_amd64.deb` |

Install a downloaded file with `sudo apt install ./shelltrace_*.deb`.

## Arch Linux / Manjaro

The upstream repository contains an AUR-ready `shelltrace-git` PKGBUILD. Build with
`cd packaging/aur && makepkg -si`, or publish metadata using
`bash packaging/aur/publish-aur.sh` after setting up the SSH public key in your AUR account.
This does **not** mean the package has already been published on aur.archlinux.org.

## Security

ShellTrace is a diagnostic tool, not a malicious-code containment system.
Host mode executes real commands; sandbox mode uses Bubblewrap and cannot guarantee
safety against hostile binaries.

## Build provenance

Release packages are created by the [Linux packages CI workflow](https://github.com/hamdyaea/ShellTrace/actions/workflows/packages.yml)
in separate native distro containers and installation-tested. Arch package syntax and
build are also tested in a real Arch Linux container.
