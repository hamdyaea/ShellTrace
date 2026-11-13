# Contributing

ShellTrace is a GPLv3-or-later community project. Issues and pull requests for correct Linux syscall interpretation, evidence quality, hardening, packaging and documentation are welcome.

## Local checks

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

Please include regression tests for changes to syscall parsing and snapshot comparison. Reports must distinguish **observed syscalls**, **confirmed snapshot differences**, and **uncertainty**. Do not claim complete change coverage or sandbox-grade protection without independent evidence and a clearly described threat model.

Contributions are accepted under the existing **GPL-3.0-or-later** license; include required provenance for third-party material.
