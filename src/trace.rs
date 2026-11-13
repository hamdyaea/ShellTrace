use crate::report::{FileOperation, NetworkEvent, Observations, ProcessEvent, SensitiveAccess};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn parse_dir(dir: &Path, max_events: usize) -> (Observations, Vec<String>) {
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
        for line in BufReader::new(file).lines() {
            let line = match line {
                Ok(line) => line,
                Err(err) => { warnings.push(format!("bad trace line: {err}")); break; }
            };
            if events_count(&events) >= max_events {
                events.truncated = true;
                break;
            }
            parse_line(pid, &line, &mut events);
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

fn parse_line(pid: u32, line: &str, out: &mut Observations) {
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
        out.network_events.push(NetworkEvent { pid, syscall: syscall.to_string(), address, result: result.clone() });
    }
    if matches!(syscall, "execve" | "execveat" | "clone" | "clone3" | "fork" | "vfork") {
        let executable = if syscall.starts_with("exec") { quoted.first().cloned() } else { None };
        out.process_events.push(ProcessEvent { pid, syscall: syscall.to_string(), executable, result: result.clone() });
    }
    if is_mutating_file_call(syscall, args) {
        out.filesystem_operations.push(FileOperation {
            pid,
            syscall: syscall.to_owned(),
            paths: if quoted.len() > 3 { quoted[..3].to_vec() } else { quoted.clone() },
            result: result.clone(),
        });
    }
    if is_path_call(syscall) {
        for candidate in &quoted {
            if is_sensitive(candidate) {
                out.sensitive_accesses.push(SensitiveAccess {
                    pid, syscall: syscall.to_string(), path: candidate.to_owned(), result: result.clone(),
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
        parse_line(31, r#"openat(AT_FDCWD, "/tmp/new", O_WRONLY|O_CREAT|O_TRUNC, 0666) = 3</dev/null>"#, &mut o);
        parse_line(31, r#"unlink("/tmp/old") = -1 ENOENT (No such file or directory)"#, &mut o);
        assert_eq!(o.filesystem_operations.len(), 2);
        assert_eq!(o.filesystem_operations[0].paths[0], "/tmp/new");
        assert!(o.filesystem_operations[1].result.starts_with("-1"));
    }

    #[test]
    fn detects_sensitive_and_network() {
        let mut o = Observations::default();
        parse_line(42, r#"openat(AT_FDCWD, "/home/u/.ssh/id_ed25519", O_RDONLY) = 3"#, &mut o);
        parse_line(42, r#"connect(3, {sa_family=AF_INET, sin_port=htons(443), sin_addr=inet_addr("1.2.3.4")}, 16) = 0"#, &mut o);
        assert_eq!(o.sensitive_accesses.len(), 1);
        assert_eq!(o.network_events.len(), 1);
        assert!(o.network_events[0].address.contains("443"));
    }

    #[test]
    fn ignores_incomplete_lines() {
        let mut o = Observations::default();
        parse_line(1, "openat(AT_FDCWD, <unfinished ...>", &mut o);
        assert_eq!(events_count(&o), 0);
    }

    #[test]
    fn does_not_read_secret_buffers_from_non_path_calls() {
        let mut o = Observations::default();
        parse_line(12, "sendto(3, \"secret\", 6, 0, NULL, 0) = 6", &mut o);
        assert_eq!(events_count(&o), 0);
    }
}
