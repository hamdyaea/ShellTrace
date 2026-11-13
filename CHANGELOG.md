# Changelog

## 0.1.0 — 2026-11-13

- Initial Rust CLI for Linux, with `run` and Bubblewrap-based `sandbox` modes.
- Before/after filesystem diff with metadata and SHA-256 comparison.
- Process, path and network syscall evidence via `strace -ff`.
- Heuristic sensitive-path detection and JSON schema version 1.
- Test suite, GitHub Actions CI, English/French documentation, security model.

**Initial release status:** source implementation. Platform-specific integration testing is required before describing a build as production-ready.
