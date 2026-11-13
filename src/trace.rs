use crate::report::{
    FileOperation, NetworkEvent, Observations, ProcessEvent, SensitiveAccess,
    ORIGIN_COMMAND, ORIGIN_SANDBOX_SETUP, ORIGIN_UNKNOWN,
};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// The origin of an observed syscall is conservative: only a successful execve
/// of the target command establishes "command" in sandbox mode. Unclassified
/// descendants are tagged "unknown", not wrongly reported as trusted setup.
pub fn parse_dir(dir: &Path, max_events: usize, sandbox: bool) -> (Observations, Vec<String>) {
    let mut events = Observations::default();
    let mut warnings = Vec::new();
    let paths = match fs::read_dir(dir) {
        Ok(paths) => paths,
        Err(err) => return (events, vec![format!("cannot read strace logs: {err}")]),
    };
    let mut log_paths = Vec::new();
    for entry in paths {
        let Ok(entry) = entry else { continue };
        if entry.file_name().to_string_lossy().starts_with("trace.") {
            log_paths.push(entry.path());
        }
    }
    log_paths.sort();
    if log_paths.is_empty() {
        warnings.push("No strace event files found; syscall coverage is unavailable".into());
    }
    for path in log_paths {
        let pid = path.file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| s.rsplit_once('.'))
            .and_then(|(_, n)| n.parse::<u32>().ok())
            .unwrap_or(0);
        let file = match fs::File::open(&path) {
            Ok(f) => f,
            Err(err) => { warnings.push(format!("cannot read {}: {err}", path.display())); continue; }
        };
        let mut origin = if sandbox { ORIGIN_UNKNOWN } else { ORIGIN_COMMAND };
        for line in BufReader::new(file).lines() {
            let line = match line {
                Ok(line) => line,
                Err(err) => { warnings.push(format!("bad trace line: {err}")); break; }
            };
            if events_count(&events) >= max_events {
                events.truncated = true;
                break;
            }
            if sandbox {
                if let Some(exe) = successful_execve(&line) {
                    origin = if is_bubblewrap(&exe) {
                        ORIGIN_SANDBOX_SETUP
                    } else {
                        ORIGIN_COMMAND
                    };
                }
            }
            parse_line(pid, &line, &mut events, origin);
        }
        if events.truncated { break; }
    }
    if events.truncated {
        warnings.push(format!("Event count limit ({max_events}) reached; raw traces or later events omitted"));
    }
    (events, warnings)
}

fn events_count(o: &Observations) -> usize {
    o.filesystem_operations.len() + o.process_events.len() + o.network_events.len() + o.sensitive_accesses.len()
}

fn is_bubblewrap(path: &str) -> bool {
    path.rsplit('/').next().is_some_and(|name| name == "bwrap")
}

fn successful_execve(line: &str) -> Option<String> {
    let (call, result) = line.rsplit_once(" = ")?;
    if !matches!(result.trim(), "0") {
        return None;
    }
    if !(call.starts_with("execve(") || call.starts_with("execveat(")) {
        return None;
    }
    quoted_strings(call).into_iter().next()
}

fn network_category(address: &str) -> &'static str {
    if address.contains("AF_INET6") || address.contains("AF_INET") {
        "internet"
    } else if address.contains("AF_NETLINK") {
        "kernel_ipc"
    } else if address.contains("AF_UNIX") || address.contains("AF_LOCAL") {
        "local_ipc"
    } else {
        "unknown"
    }
}

fn parse_line(pid: u32, line: &str, out: &mut Observations, origin: &str) {
    // strace -ff normally emits complete calls, but interrupted/resumed calls are
    // deliberately not reconstructed; they are excluded rather than invented.
    if line.contains("<unfinished ...>") || line.contains("resumed>") { return; }
    let Some((call, result)) = line.rsplit_once(" = ") else { return };
    let Some((syscall, args)) = call.split_once('(') else { return };
    if syscall.is_empty() || !syscall.bytes().all(|b| b.is_ascii_lowercase() || b == b'_' || b.is_ascii_digit()) {
        return;
    }
    let result = result.trim().to_string();
    let quoted = quoted_strings(args);
    if matches!(syscall, "connect" | "bind" | "listen" | "accept" | "accept4") {
        let address = if matches!(syscall, "connect" | "bind") {
            network_address(args)
        } else {
            String::new()
        };
        let category = network_category(&address).to_string();
        let succeeded = !result.starts_with("-1") && !result.starts_with('?');
        out.network_events.push(NetworkEvent {
            pid, syscall: syscall.to_string(), address, result: result.clone(),
            origin: origin.to_string(), category, succeeded,
        });
    }
    if matches!(syscall, "execve" | "execveat" | "clone" | "clone3" | "fork" | "vfork") {
        let executable = if syscall.starts_with("exec") { quoted.first().cloned() } else { None };
        out.process_events.push(ProcessEvent {
            pid, syscall: syscall.to_string(), executable, result: result.clone(),
            origin: origin.to_string(),
        });
    }
    if is_mutating_file_call(syscall, args) {
        out.filesystem_operations.push(FileOperation {
            pid,
            syscall: syscall.to_owned(),
            paths: if quoted.len() > 3 { quoted[..3].to_vec() } else { quoted.clone() },
            result: result.clone(),
            origin: origin.to_string(),
        });
    }
    if is_path_call(syscall) {
        for candidate in &quoted {
            if is_sensitive(candidate) {
                out.sensitive_accesses.push(SensitiveAccess {
                    pid, syscall: syscall.to_string(), path: candidate.to_owned(), result: result.clone(),
                    origin: origin.to_string(),
                });
            }
        }
    }
}

fn is_path_call(call: &str) -> bool {
    matches!(call,
        "open" | "openat" | "openat2" | "creat" | "stat" | "statx" | "lstat" | "newfstatat" | "access" | "faccessat" | "faccessat2" |
        "readlink" | "readlinkat" | "execve" | "execveat" | "unlink" | "unlinkat" | "rename" | "renameat" | "renameat2" |
        "chmod" | "fchmodat" | "chown" | "fchownat" | "truncate" | "mkdir" | "mkdirat" | "rmdir" | "link" | "linkat" |
        "symlink" | "symlinkat" | "utimensat" | "mknod" | "mknodat")
}

fn is_mutating_file_call(call: &str, args: &str) -> bool {
    if matches!(call, "open" | "openat" | "openat2") {
        return ["O_WRONLY", "O_RDWR", "O_CREAT", "O_TRUNC", "O_TMPFILE"].iter().any(|flag| args.contains(flag));
    }
    matches!(call,
        "creat" | "unlink" | "unlinkat" | "rename" | "renameat" | "renameat2" |
        "mkdir" | "mkdirat" | "rmdir" | "chmod" | "fchmodat" | "fchmodat2" |
        "chown" | "lchown" | "fchownat" | "truncate" | "link" | "linkat" |
        "symlink" | "symlinkat" | "utimensat" | "mknod" | "mknodat" | "setxattr" | "lsetxattr" |
        "removexattr" | "lremovexattr")
}

fn is_sensitive(path: &str) -> bool {
    let lowered = path.to_ascii_lowercase();
    lowered.contains("/.ssh/") || lowered.ends_with("/.ssh") ||
    lowered.contains("/.aws/") || lowered.ends_with("/.aws") ||
    lowered.contains("/.gnupg/") || lowered.ends_with("/.gnupg") ||
    lowered.contains("/.kube/") || lowered.ends_with("/.kube") ||
    lowered.contains("/.config/gcloud/") ||
    lowered == "/etc/shadow" || lowered == "/etc/gshadow" ||
    (lowered.starts_with("/proc/") && lowered.ends_with("/environ")) ||
    (lowered.ends_with("/.env") || lowered == ".env")
}

// Returns the quoted strings in syscall arguments without preserving the full
// syscall line, which can include sensitive data from unrelated buffer syscalls.
fn quoted_strings(input: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut inside = false;
    let mut escaped = false;
    for c in input.chars() {
        if !inside {
            if c == '"' { inside = true; current.clear(); }
        } else if escaped {
            current.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            result.push(current.clone());
            inside = false;
        } else {
            current.push(c);
        }
    }
    result
}

fn network_address(args: &str) -> String {
    // Return only sockaddr-like material; never copy send/recv payloads.
    let start = args.find("sa_family=").or_else(|| args.find("sun_path="));
    match start {
        Some(start) => {
            let rest = &args[start..];
            rest.chars().take(250).collect()
        }
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_open_write_and_failed_attempt() {
        let mut o = Observations::default();
        parse_line(31, r#"openat(AT_FDCWD, "/tmp/new", O_WRONLY|O_CREAT|O_TRUNC, 0666) = 3</dev/null>"#, &mut o, ORIGIN_COMMAND);
        parse_line(31, r#"unlink("/tmp/old") = -1 ENOENT (No such file or directory)"#, &mut o, ORIGIN_COMMAND);
        assert_eq!(o.filesystem_operations.len(), 2);
        assert_eq!(o.filesystem_operations[0].paths[0], "/tmp/new");
        assert!(o.filesystem_operations[1].result.starts_with("-1"));
    }

    #[test]
    fn detects_sensitive_and_network() {
        let mut o = Observations::default();
        parse_line(42, r#"openat(AT_FDCWD, "/home/u/.ssh/id_ed25519", O_RDONLY) = 3"#, &mut o, ORIGIN_COMMAND);
        parse_line(42, r#"connect(3, {sa_family=AF_INET, sin_port=htons(443), sin_addr=inet_addr("1.2.3.4")}, 16) = 0"#, &mut o, ORIGIN_COMMAND);
        assert_eq!(o.sensitive_accesses.len(), 1);
        assert_eq!(o.network_events.len(), 1);
        assert!(o.network_events[0].address.contains("443"));
    }

    #[test]
    fn ignores_incomplete_lines() {
        let mut o = Observations::default();
        parse_line(1, "openat(AT_FDCWD, <unfinished ...>", &mut o, ORIGIN_COMMAND);
        assert_eq!(events_count(&o), 0);
    }

    #[test]
    fn does_not_read_secret_buffers_from_non_path_calls() {
        let mut o = Observations::default();
        parse_line(12, "sendto(3, \"secret\", 6, 0, NULL, 0) = 6", &mut o, ORIGIN_COMMAND);
        assert_eq!(events_count(&o), 0);
    }

    #[test]
    fn separates_kernel_ipc_from_external_network() {
        let mut o = Observations::default();
        parse_line(1, "bind(3, {sa_family=AF_NETLINK, nl_pid=1}, 12) = 0", &mut o, ORIGIN_SANDBOX_SETUP);
        parse_line(2, r#"connect(3, {sa_family=AF_INET6, sin6_port=htons(443)}, 28) = -1 ENETUNREACH"#, &mut o, ORIGIN_COMMAND);
        assert_eq!(o.network_events[0].category, "kernel_ipc");
        assert_eq!(o.network_events[0].origin, "sandbox_setup");
        assert_eq!(o.network_events[1].category, "internet");
        assert!(!o.network_events[1].succeeded);
    }

    #[test]
    fn identifies_successful_execve_only() {
        assert_eq!(successful_execve(r#"execve("/usr/bin/bwrap", ["bwrap"], 0x7ff) = 0"#).as_deref(), Some("/usr/bin/bwrap"));
        assert_eq!(successful_execve(r#"execve("/bin/sh", ["sh"], 0x7ff) = -1 ENOENT"#), None);
        assert_eq!(successful_execve(r#"clone(child_stack=NULL) = 123"#), None);
        assert!(is_bubblewrap("/usr/bin/bwrap"));
        assert!(!is_bubblewrap("/bin/sh"));
    }

    #[test]
    fn parser_separates_sandbox_and_command_origins() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("trace.100"),
            "execve(\"/usr/bin/bwrap\", [\"bwrap\"], 0x00) = 0\nopenat(AT_FDCWD, \"/tmp/setup\", O_WRONLY) = 3\n").unwrap();
        fs::write(dir.path().join("trace.101"),
            "execve(\"/bin/sh\", [\"sh\"], 0x00) = 0\nopenat(AT_FDCWD, \"/tmp/target\", O_CREAT|O_WRONLY) = 3\n").unwrap();
        let (o, errors) = parse_dir(dir.path(), 100, true);
        assert!(errors.is_empty());
        assert_eq!(o.filesystem_operations.len(), 2);
        assert_eq!(o.filesystem_operations[0].origin, "sandbox_setup");
        assert_eq!(o.filesystem_operations[1].origin, "command");
    }
}
}
