# Changelog

## 0.3.0 — 2026-11-13

- Adopt the selected shield badge for app window, launcher and GitHub documentation; ship dark/light wordmark variants.
- Add persistent Qt6 **Dark / Light / System** appearance selector, with **Dark** as the default.
- Add source AUR package `shelltrace-git` and SSH publication script for Arch/Manjaro.
- Build distribution-specific `.deb` packages for Debian 12/13 and Ubuntu 24.04/26.04 through GitHub Actions.
- Package both Rust CLI and Qt6 GUI with the application icon and license.


## 0.2.0 — 2026-11-13

- Fix GUI installation on Manjaro when old `~/.cargo/bin` shims shadow up-to-date `/usr/bin` Cargo/Rust; display the actual versions used before building.

- Add native Qt6 Widgets graphical desktop application backed by the Rust CLI, plus a per-user application-menu installer.
- Show filesystem changes with structured before/after file size, permissions, target and SHA-256 when available.
- Add conservative syscall origin classification (`command`, `sandbox_setup`, `unknown`).
- Distinguish Internet socket operations from AF_NETLINK/kernel IPC and Unix sockets.
- Improve terminal reports with concise, color-aware status, `--verbose` and `--no-color`.
- Upgrade JSON schema version to **2** (additive fields; consumers should check the schema version).
- Expand regression tests, add loopback TCP integration check and Qt6 CI build/smoke tests.


## 0.1.2 — 2026-11-13

- Fix `bwrap: Can\'t create file /media: Read-only file system` on systems without `/media`.
- Skip optional Bubblewrap tmpfs mounts unless their destination exists as an actual directory in the host root.
- Add regression tests for missing optional mount directories.
- Commit `Cargo.lock` for reproducible builds.


## 0.1.1 — 2026-11-13

- Fix the sandbox workspace mount failure on read-only root (Bubblewrap error: `Can't create file /work: Read-only file system`).
- Mount a private `/tmp` first, then bind the working directory at `/tmp/shelltrace-work`.
- Synchronize reported sandbox scope with its real mount point.
- Add a sandbox integration regression test when user namespaces are available.
- Host `run` mode and the JSON schema remain backward compatible.

## 0.1.0 — 2026-11-13

- Initial Rust CLI for Linux, with `run` and Bubblewrap-based `sandbox` modes.
- Before/after filesystem diff with metadata and SHA-256 comparison.
- Process, path and network syscall evidence via `strace -ff`.
- Heuristic sensitive-path detection and JSON schema version 1.
- Test suite, GitHub Actions CI, English/French documentation, security model.

**Initial release status:** source implementation. Platform-specific integration testing is required before describing a build as production-ready.
