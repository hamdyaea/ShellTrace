# ShellTrace — Qt6 desktop GUI

Native **Qt6 Widgets** desktop interface for the **Rust ShellTrace engine**. The GUI is a small optional C++/Qt6 program which invokes the Rust CLI using `QProcess`, and renders structured JSON reports. The analysis engine remains entirely Rust. No Qt dependency is required to use the CLI.

## Features

- Friendly launch form with executable, arguments and engine selector.
- Sandbox mode selected by default; host mode requires explicit confirmation.
- Asynchronous execution with progress indicator and stop action.
- File changes dashboard including SHA-256 and metadata **before/after**.
- File syscall, process, network, and sensitive path views.
- Filter command vs Bubblewrap setup vs unclassified events, plus text search.
- View log, open existing JSON, and save a JSON report.
- **English-only UI**, independent of the desktop language and locale.

## Arch Linux / Manjaro / CachyOS

```bash
sudo pacman -S --needed rust strace bubblewrap cmake qt6-base
cd ~/ShellTrace
bash scripts/build-gui.sh
./build/gui/shelltrace-gui
```

## Debian / Ubuntu

```bash
sudo apt update
sudo apt install cargo rustc strace bubblewrap cmake g++ qt6-base-dev
cargo build --release --locked
cmake -S gui -B build/gui -DCMAKE_BUILD_TYPE=Release
cmake --build build/gui --parallel
./build/gui/shelltrace-gui
```

Your distribution's Rust version must be recent enough for the locked dependencies; use a newer toolchain if required.

## Install in the Linux application menu

After installing Qt6 dependencies, the build script checks the active Rust and Cargo versions. If an outdated rustup installation under `~/.cargo/bin` shadows an up-to-date Arch/Manjaro installation under `/usr/bin`, the script automatically selects the newer system toolchain.

After installing Qt6 dependencies:

```bash
bash scripts/install-gui.sh
```

This builds the Rust engine and GUI and installs both under `~/.local/bin` plus an application menu entry under `~/.local/share/applications`. No root privileges are needed. To remove:

```bash
bash scripts/install-gui.sh --uninstall
```

## Usage

Leave **Sandbox** selected to test a command without granting it normal host write access. The example uses `/bin/sh` and an explicit `-c "echo ShellTrace > gui-demo.txt"` argument. `QProcess` passes an argument vector and does not insert an implicit shell. Host mode actually modifies the host, possibly outside the watched directory.

Click **Start analysis** and review the summary and table tabs. Use the origin filter to separate target-command syscalls from Bubblewrap setup, or show unclassified events. Double-click any event for complete row values. Use **Open JSON report** to inspect an existing report without rerunning the command.

Reports can contain confidential paths and command arguments; review before sharing. A Bubblewrap sandbox is **not** a replacement for a disposable VM for malicious samples. Stopping the tracer may leave background descendants.

## Qt-only smoke test

```bash
QT_QPA_PLATFORM=offscreen ./build/gui/shelltrace-gui --self-test
```

This checks that the Qt interface starts in headless mode. It does not execute a command or establish sandbox security.

## Appearance

Use the **Dark / Light / System** selector in the top-right corner. **Dark** is the initial default. Your selected mode is persisted using Qt `QSettings` in your user profile; System restores the desktop palette. The selected shield badge is used for the main window and launcher.

## Theme contrast (v0.3.1)

Dark mode now uses explicit colors for text and widget surfaces instead of relying on the previous widget stylesheet's palette values. The main text contrast is approximately **13.8:1** (`#F4F8FF` on `#1D293A`). Dropdown popups, tables, buttons, tooltips and selection states use the same coherent palette. Switching to **System** removes the explicit dark/light color overrides and restores the desktop palette.

Headless CI regression check:

```bash
QT_QPA_PLATFORM=offscreen ./build/gui/shelltrace-gui --self-test-contrast
```

## English-only interface (v0.3.2)

All ShellTrace-owned Qt6 labels, menus, warnings, dialogs and status messages are presented in **English**, regardless of the operating system language. Native file chooser dialogs are disabled in favor of the Qt dialogs to avoid host-localized button text. The GUI sets Qt's default locale to English, without changing the locale of the command being traced: the command's own stdout/stderr may still appear in the language selected by that command or by the host.

Regression check (also run in CI with `LANG=fr_CH.UTF-8`):

```bash
QT_QPA_PLATFORM=offscreen LANG=fr_CH.UTF-8 ./build/gui/shelltrace-gui --self-test-english
```
