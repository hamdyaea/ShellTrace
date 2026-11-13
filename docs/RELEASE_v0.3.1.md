# ShellTrace v0.3.1 — high-contrast dark theme fix

This patch release fixes the unreadable **black text on dark gray** regression
in ShellTrace's native Qt6 desktop interface, reported on Manjaro.

## Changes

- Dark theme now uses explicit, high-contrast text and surfaces instead of
  depending on Qt's platform palette interpretation.
- Inputs, comboboxes and their dropdown lists, tab labels, table rows and
  headers, buttons, notices, tooltips and status messages all use consistent
  foreground and background colors.
- Selected text is explicitly colored for readability.
- Switching back to **System** restores the original desktop palette;
  **Dark** remains the default on first launch, and preferences persist.
- Add headless Qt6 contrast-regression checks for Dark, Light and System.

## Packages

This release provides separate, native `.deb` packages for:
- Debian 12 (amd64)
- Debian 13 (amd64)
- Ubuntu 24.04 (amd64)
- Ubuntu 26.04 (amd64)

The Arch Linux `shelltrace-git` AUR source package automatically follows
upstream Git; `PKGBUILD` and `.SRCINFO` for version 0.3.1 are provided
in `packaging/aur/`.

## Update

On Manjaro, for a user-local installation:

```bash
cd ~/ShellTrace
git pull --ff-only
bash scripts/install-gui.sh
~/.local/bin/shelltrace-gui
```

On Debian/Ubuntu, download the matching `.deb` and use
`sudo apt install ./shelltrace_0.3.1-1_*.deb`.

## Validation

GitHub Actions tests Rust, Qt6 (including the headless theme contrast check),
install-tests each native Debian/Ubuntu `.deb`, and compiles the AUR source
package inside an Arch Linux container. The CI checks do not substitute for
on-screen confirmation in every Linux desktop environment.
