use crate::report::{FileChange, FileState, Report, ORIGIN_COMMAND, ORIGIN_SANDBOX_SETUP};
use std::io::{self, IsTerminal};

fn tint<'a>(value: &'a str, color: &'static str, enabled: bool) -> String {
    if enabled { format!("\x1b[{color}m{value}\x1b[0m") } else { value.to_owned() }
}

fn stat(state: Option<&FileState>) -> String {
    match state {
        None => "—".to_owned(),
        Some(s) => {
            let hash = s.sha256.as_deref().map(|h| format!(" sha256={}", &h[..h.len().min(12)])).unwrap_or_default();
            let target = s.symlink_target.as_deref().map(|t| format!(" -> {t}")).unwrap_or_default();
            format!("{} {} B mode={:04o}{}{}", s.file_type, s.size_bytes, s.mode, hash, target)
        }
    }
}

fn show_change(change: &FileChange, colored: bool) {
    let (mark, tone) = match change.kind.as_str() {
        "created" => ("+", "32"),
        "deleted" => ("−", "31"),
        "modified" | "type_changed" => ("~", "33"),
        "metadata_changed" => ("m", "36"),
        _ => ("?", "35"),
    };
    println!("  {} {:<18} {}", tint(mark, tone, colored), change.kind, change.path);
    if change.before.is_some() && change.after.is_some() {
        println!("    before: {}", stat(change.before.as_ref()));
        println!("    after : {}", stat(change.after.as_ref()));
    } else if change.after.is_some() {
        println!("    after : {}", stat(change.after.as_ref()));
    } else if change.before.is_some() {
        println!("    before: {}", stat(change.before.as_ref()));
    }
    if change.kind == "possibly_modified" {
        println!("    Uncertain: {}", change.evidence);
    }
}

pub fn print_summary(report: &Report, verbose: bool, plain: bool) {
    let colored = !plain && io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    let ok = report.exit_code == 0;
    println!();
    println!("{}", tint("┌──────────────────────────────────────────────────┐", "36", colored));
    println!("{}  ShellTrace {}  •  {}",
        tint("│", "36", colored),
        env!("CARGO_PKG_VERSION"),
        if report.mode == "sandbox" {"Sandbox trial"} else {"Host observation"});
    println!("{}  {} · {} ms · status {}",
        tint("│", "36", colored),
        if ok { tint("Completed", "32", colored) } else { tint("Command failed", "31", colored) },
        report.duration_ms,
        report.exit_code);
    println!("{}", tint("└──────────────────────────────────────────────────┘", "36", colored));
    println!("Watched directory: {}", report.scope);
    let (mut created, mut deleted, mut changed, mut uncertain) = (0, 0, 0, 0);
    for change in &report.filesystem_changes {
        match change.kind.as_str() {
            "created" => created += 1,
            "deleted" => deleted += 1,
            "possibly_modified" => uncertain += 1,
            _ => changed += 1,
        }
    }
    println!("Files  {} created  ·  {} changed  ·  {} deleted  ·  {} uncertain", created, changed, deleted, uncertain);
    let obs = &report.observations;
    let command = |origin: &str| origin == ORIGIN_COMMAND;
    let command_files = obs.filesystem_operations.iter().filter(|e| command(&e.origin)).count();
    let command_processes = obs.process_events.iter().filter(|e| command(&e.origin) && e.executable.is_some()).count();
    let internet = obs.network_events.iter().filter(|e| command(&e.origin) && e.category == "internet").count();
    let local = obs.network_events.iter().filter(|e| command(&e.origin) && e.category != "internet").count();
    let runtime = obs.filesystem_operations.iter().filter(|e| e.origin == ORIGIN_SANDBOX_SETUP).count()
        + obs.network_events.iter().filter(|e| e.origin == ORIGIN_SANDBOX_SETUP).count()
        + obs.process_events.iter().filter(|e| e.origin == ORIGIN_SANDBOX_SETUP).count();
    println!("Trace  {} file operations · {} executed processes · {} Internet socket calls · {} local/other socket calls",
        command_files, command_processes, internet, local);
    if report.mode == "sandbox" {
        println!("Sandbox setup: {runtime} internal events (separated from command evidence)");
        println!("Unclassified events may also occur; see JSON origin fields.");
    }
    println!();
    if !report.filesystem_changes.is_empty() {
        println!("{}", tint("FILES CHANGED (snapshot comparison)", "1;36", colored));
        for change in report.filesystem_changes.iter().take(if verbose { 200 } else { 30 }) {
            show_change(change, colored);
        }
        if report.filesystem_changes.len() > if verbose { 200 } else { 30 } {
            println!("  Additional changes are available in the JSON report.");
        }
    } else {
        println!("{}", tint("No filesystem difference observed in the watched directory.", "33", colored));
    }
    if verbose {
        println!();
        println!("{}", tint("COMMAND FILE OPERATIONS (attempts, not confirmed writes)", "1;36", colored));
        for e in obs.filesystem_operations.iter().filter(|e| command(&e.origin)).take(50) {
            println!("  pid={} {} {:?} → {}", e.pid, e.syscall, e.paths, e.result);
        }
        println!("{}", tint("PROCESSES", "1;36", colored));
        for e in obs.process_events.iter().filter(|e| command(&e.origin)).take(50) {
            println!("  pid={} {} {:?} → {}", e.pid, e.syscall, e.executable, e.result);
        }
        println!("{}", tint("NETWORK (not proof of remote communication)", "1;36", colored));
        for e in obs.network_events.iter().filter(|e| command(&e.origin)).take(50) {
            println!("  pid={} {} [{}] {} → {}", e.pid, e.syscall, e.category, e.address, e.result);
        }
        println!("{}", tint("SENSITIVE PATHS (heuristics only)", "1;36", colored));
        for e in obs.sensitive_accesses.iter().filter(|e| command(&e.origin)).take(50) {
            println!("  pid={} {} {} → {}", e.pid, e.syscall, e.path, e.result);
        }
        if report.mode == "sandbox" {
            println!("{}", tint("SANDBOX SETUP (internal syscalls)", "1;36", colored));
            for e in obs.filesystem_operations.iter().filter(|e| e.origin == ORIGIN_SANDBOX_SETUP).take(12) {
                println!("  pid={} {} {:?} → {}", e.pid, e.syscall, e.paths, e.result);
            }
        }
    } else {
        println!("Tip: add --verbose before -- to inspect per-command syscalls.");
    }
    if obs.truncated { eprintln!("Warning: syscall report was truncated."); }
    for warning in report.warnings.iter().take(10) {
        eprintln!("{} {warning}", tint("Warning:", "33", colored));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn state_prints_human_readable_octal_mode() {
        let state = FileState {
            file_type: "file".into(), size_bytes: 42, mode: 0o644,
            sha256: Some("0123456789abcdef".into()), symlink_target: None,
        };
        assert!(stat(Some(&state)).contains("mode=0644"));
        assert!(stat(Some(&state)).contains("0123456789ab"));
    }
}
