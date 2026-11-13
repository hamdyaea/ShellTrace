# ShellTrace

**Know what a command really does before you trust it.**

[![CI](https://github.com/hamdyaea/ShellTrace/actions/workflows/ci.yml/badge.svg)](https://github.com/hamdyaea/ShellTrace/actions/workflows/ci.yml)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/License-GPLv3%2B-blue.svg)](LICENSE)

**ShellTrace** is an open-source **Rust CLI for Linux** that observes how a command behaves and documents its effects with evidence. It combines **before/after filesystem snapshots** with **`strace` syscall observations**. It does **not** claim to see every change to a machine.

**Two execution modes:**

- **`run` — observe on the actual host.** The command runs with your normal privileges and can **modify the host**. Before/after comparisons cover a directory you choose, while tracing can reveal paths referenced outside it.
- **`sandbox` — trial execution using Bubblewrap.** Runs in a separate namespace, with a read-only bind of the host root, a writable ephemeral `/tmp/shelltrace-work`, no shared network namespace, and masked home/runtime mount points. Not a hardened security boundary or a guarantee against all side effects.

## What ShellTrace reports

| Signal | Method | Evidence and caveats |
|---|---|---|
| Created/deleted paths | Before/after snapshots | Confirmed **within the watched directory** at snapshot times |
| Modified files | SHA-256, file size, modes, timestamps | Hash comparison confirms content differences for files within the configured size limit |
| File operations | `strace` path syscalls | Observed *attempts*; a successful call does not guarantee durable on-disk changes |
| Child processes | `execve`, `clone`, `fork`, etc. | Observed process-related syscall results |
| Network operations | `connect`, `bind`, `listen`, `accept` | Observed socket operations; **not** a packet capture or proof of successful application-layer communication |
| Sensitive path access | `.ssh`, `.aws`, `.gnupg`, `.kube`, `/etc/shadow`, `.env`, etc. | Heuristic path matching, not a security verdict |

ShellTrace does **not** reconstruct operations from unfinished/resumed `strace` lines, intercept kernel-only side effects, see writes to other machines, or inventory all file changes outside `--scope`. Background descendants may outlive the traced leader. Files can change concurrently for unrelated reasons, so a snapshot diff shows **what changed**, not necessarily which process caused it. Tracing is best-effort, not proof of causality.

## Installation

**Requirements:** Linux, Rust toolchain (`cargo`), and [`strace`](https://strace.io/). The `sandbox` subcommand additionally requires [`bubblewrap`](https://github.com/containers/bubblewrap) (`bwrap`) and working unprivileged user namespaces.

On Arch Linux, CachyOS, Manjaro:

```bash
sudo pacman -S --needed rust strace bubblewrap
```

On Debian/Ubuntu:

```bash
sudo apt update
sudo apt install cargo rustc strace bubblewrap
```

> Rust **1.76+** is required by the manifest; distribution `cargo` packages may be older. Use rustup where needed.

Build from source:

```bash
git clone https://github.com/hamdyaea/ShellTrace.git
cd ShellTrace
cargo build --release
./target/release/shelltrace --help
```

## Quick start — host observation

Create a disposable test directory:

```bash
mkdir -p /tmp/shelltrace-demo
shelltrace run \
  --scope /tmp/shelltrace-demo \
  --json /tmp/shelltrace-report.json \
  -- /bin/sh -c 'printf "hello\n" > /tmp/shelltrace-demo/new.txt'
```

Or use the locally built executable with `./target/release/shelltrace` instead of `shelltrace`. **Commands in `run` mode are executed for real.**

Typical output (illustrative, not a verified run):

```text
ShellTrace v0.1.2 | host | exit=0 | 21 ms
Scope: /tmp/shelltrace-demo
Filesystem diff: 1 changes
  created          new.txt (path appeared)
Syscall observations: 1 filesystem operations, 1 process events, 0 network events, 0 sensitive accesses
JSON report: /tmp/shelltrace-report.json
```

## Quick start — sandbox trial

```bash
shelltrace sandbox \
  --json /tmp/shelltrace-sandbox.json \
  --export-workspace /tmp/shelltrace-results \
  -- /bin/sh -c 'echo sandbox-test > result.txt; cat result.txt'
```

To copy a project into the temporary workspace before execution:

```bash
shelltrace sandbox --seed ./example-project --json /tmp/try.json -- \
  /bin/sh -c 'echo changed > result.txt'
```

- `/tmp/shelltrace-work` is writable and its contents are inspected after the command finishes.
- The shell's cwd is `/tmp/shelltrace-work`. Absolute paths from the host may exist read-only inside the sandbox, with selected sensitive directories masked.
- Network namespaces are separated. The command usually cannot reach external networks.
- Optional sandbox masks (for example `/media` and `/mnt`) are mounted only if those directories exist on the host. Missing mount targets are skipped rather than triggering a Bubblewrap failure.
- The sandbox can fail where user namespaces, mount namespaces, or ptrace are disallowed. Failures are not silently bypassed.
- Exporting a workspace is optional; `--export-workspace` must point to a **new directory**.

**Warning:** This example sandbox is designed for **inspection, not hostile-code containment**. It does not use seccomp; portions of the host root are still readable. Do not execute malicious or untrusted software expecting full isolation. Use a disposable VM for high-risk samples. See [SECURITY.md](SECURITY.md).

## CLI synopsis

```text
shelltrace run [--scope DIR] [--allow-root-scope] [--json FILE]
               [--keep-trace DIR] [--max-hash-mib N] -- COMMAND [ARG...]

shelltrace sandbox [--seed DIR] [--export-workspace NEW_DIR]
                   [--json FILE] [--keep-trace DIR] [--max-hash-mib N]
                   -- COMMAND [ARG...]
```

`--keep-trace` saves the **raw syscall logs**, which can reveal file paths, command arguments, addresses and other confidential information. The default deletes them after generating the report. Files above `--max-hash-mib` are not hashed: same-size modifications may not be detected reliably. CLI and JSON reports have an event limit of 20,000 observations; the report includes `observations.truncated` when reached.

ShellTrace returns the executed command's exit status (or `1` for program errors). A nonzero exit status does not itself mean no files were changed.

## JSON output

The stable v1 JSON envelope contains:

```json
{
  "schema_version": 1,
  "mode": "host",
  "started_at_unix": 0,
  "duration_ms": 0,
  "command": ["/bin/true"],
  "scope": "/tmp/shelltrace-demo",
  "exit_code": 0,
  "filesystem_changes": [],
  "observations": {
    "filesystem_operations": [],
    "process_events": [],
    "network_events": [],
    "sensitive_accesses": [],
    "truncated": false
  },
  "warnings": [],
  "raw_trace_path": null
}
```

The example above illustrates the **schema**, not measurements. A trace result's `result` field contains the raw syscall return value; negative/error returns indicate failed attempts. The `filesystem_changes` array contains types such as `created`, `deleted`, `modified`, `metadata_changed`, `possibly_modified`, `type_changed`.

## Develop and contribute

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
```

See [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md), and [CHANGELOG.md](CHANGELOG.md). Please report incomplete or incorrect evidence rather than assuming absence of evidence proves absence of behavior.

Author: **Hamdy Abou El Anein** · [GitHub](https://github.com/hamdyaea) · [Daylight GNU/Linux](https://daylightlinux.ch)

Licensed under **GNU GPL version 3 or later**. See [LICENSE](LICENSE).

**Note:** Place `--keep-trace` outside the watched `--scope` directory to avoid mixing ShellTrace's own evidence files with target-command changes. The CLI rejects an obviously nested trace output directory.

## Publishing from a local checkout

The optional helper `bash scripts/publish-github.sh` creates the public `hamdyaea/ShellTrace` repository (if absent) and pushes `main`, provided `git` and an authenticated GitHub CLI (`gh auth login`) are available. It does **not** force-push. Publishing to a repository containing unrelated commits can require a manual merge first.
