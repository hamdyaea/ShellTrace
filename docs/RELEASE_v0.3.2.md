# ShellTrace v0.3.2 — English-only desktop interface

ShellTrace now presents its **entire built-in Qt6 interface in English**,
regardless of the Linux desktop language. This release also updates the
native Debian/Ubuntu binary packages and the AUR source package.

## Changes

- Remove **99 bilingual French/English GUI labels**, including application
  forms, tabs, buttons, prompts, report details, warnings and status messages.
- Always select English for the Qt6 interface; do not depend on a French,
  German or other desktop locale.
- Use Qt's English file picker dialogs instead of localized native dialogs.
- Remove the French locale-specific desktop launcher comment.
- Preserve **Dark**, **Light** and **System** appearance choices and the
  previous high-contrast dark theme fix.
- Do not alter the locale of any traced command: external command output
  may retain the language chosen by that command or by the host.
- Include a headless GUI regression test with `LANG=fr_CH.UTF-8`
  to prevent inadvertent language fallback.
- Update Rust CLI / Qt6 GUI package version to **0.3.2**.

## Downloads

Choose the matching `.deb` package for your Linux distribution:

- Debian 12 (amd64)
- Debian 13 (amd64)
- Ubuntu 24.04 (amd64)
- Ubuntu 26.04 (amd64)

To install: `sudo apt install ./shelltrace_0.3.2-1_*.deb`.

## Manjaro / Arch

Update the local source checkout:

```bash
cd ~/ShellTrace
git pull --ff-only
bash scripts/install-gui.sh
~/.local/bin/shelltrace-gui
```

The `shelltrace-git` AUR build recipe is available in
`packaging/aur/`, and tested on Arch Linux. A maintainer must publish
the recipe over their authorized SSH connection to AUR.

## Verification

GitHub Actions compiles Rust, tests Qt6 and its appearance settings,
checks English-only UI labels under a simulated French locale,
builds and install-tests the native Debian/Ubuntu packages,
and compiles the Arch source package.
