# Security model and limitations

ShellTrace is a **diagnostic and forensic-assistance tool**, not a security monitor, malicious-code detonator, or proof of complete system integrity.

## Host mode

`run` executes the command **with the user's real privileges**. It may delete data, alter packages, create persistent background processes, steal secrets, and contact remote hosts. Merely tracing it does not mitigate any of those risks. Never feed untrusted commands to host mode. Prefer disposable test trees.

## Sandbox mode

`sandbox` depends on Bubblewrap and Linux namespace behavior. The host's root filesystem is bind-mounted read-only, selected paths (`/home`, `/root`, `/run`, `/mnt`, `/media`) are masked, and `/tmp/shelltrace-work` is writable. Optional directory masks are only mounted where their destination is an existing real directory on the host; missing paths are skipped. Symlink paths are not masked by this mechanism. The network namespace is unshared; this does not guarantee all forms of local IPC or side effects are blocked. It deliberately does **not** implement a default seccomp syscall policy, and the host root still exposes data outside the masked directories. Paths such as `/etc`, `/opt`, `/srv` and `/var` may remain readable. There is no defense against kernel exploits, resource exhaustion, or malicious logic using unfiltered interfaces.

**Do not run hostile or unknown software under the assumption that ShellTrace's sandbox is sufficient.** Use a separately isolated VM with no host credentials or shared folders instead.

If Bubblewrap cannot establish namespaces, ShellTrace will surface the failure; it will **not** fall back to host execution.

## Observation accuracy

- Snapshots are point-in-time. Changes caused by other processes may be included; rapid create/delete cycles may be absent.
- File hashes are limited by `--max-hash-mib`, so large-file modifications may be inferred from metadata only or missed.
- Only the given scope (host) or `/tmp/shelltrace-work` (sandbox) has a filesystem diff. Writes to paths elsewhere are not proven by a diff.
- Ptrace/strace may be restricted by Yama, container policies, seccomp, or kernel configuration.
- Successful syscalls are not the same as durable effects. Network events are socket observations, not an indication that a remote host accepted a transaction.
- Incomplete/resumed syscall lines are not reconstructed.
- Tracing has overhead and can alter the timing of the traced command.
- Even a clean report does not prove a command is safe.

## GUI trust boundaries

The optional Qt6 GUI is a user interface to the Rust CLI; it does not implement a second sandbox. It starts the Rust program via `QProcess` with an argument vector (not a hidden shell), and requires user confirmation before host mode. It stores temporary JSON under a private temporary folder; saved reports may contain secrets. Stopping the tracer does not guarantee that all descendant processes have stopped.

Syscall origin tagging is best-effort: `unknown` never means harmless, and `sandbox_setup` means the event was observed before the target exec rather than proving a trusted process identity. Network `succeeded` is inferred from syscall return; it does not verify application-level delivery.

## Handling trace evidence

Raw strace output may include secrets, command-line arguments, filenames and addresses. Use `--keep-trace` **only** in a secured directory. JSON reports may include sensitive paths and arguments too. Treat both as confidential. Temporary traces are removed after processing by default, but a crash may leave temporary files behind.

## Disclosure

Please report vulnerabilities through a private security advisory on the GitHub repository when available. Do not paste unredacted logs containing tokens, passwords, SSH keys or customer data into public issues.
