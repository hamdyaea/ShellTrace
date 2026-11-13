# Changelog

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
