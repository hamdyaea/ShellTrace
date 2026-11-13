#![forbid(unsafe_code)]
#![cfg(target_os = "linux")]

mod report;
mod sandbox;
mod snapshot;
mod trace;

use clap::{Args, Parser, Subcommand};
use report::Report;
use snapshot::{compare, snapshot};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Parser, Debug)]
#[command(name = "shelltrace", version, about = "Trace what a Linux command does, with evidence and explicit coverage limits")]
struct Cli {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand, Debug)]
enum Action {
    /// Observe a command on the real host. THE COMMAND CAN MODIFY YOUR SYSTEM.
    Run(RunOptions),
    /// Run a command in a best-effort bubblewrap sandbox; not a security boundary.
    Sandbox(SandboxOptions),
}

#[derive(Args, Debug)]
struct Common {
    /// Save a structured JSON report (created after the command finishes).
    #[arg(long)]
    json: Option<PathBuf>,
    /// Preserve raw strace files in this directory (may contain secrets).
    #[arg(long)]
    keep_trace: Option<PathBuf>,
    /// Hash files up to this size in MiB in each snapshot (default: 16).
    #[arg(long, default_value_t = 16)]
    max_hash_mib: u64,
    /// Command and arguments, separated from options by --.
    #[arg(last = true, required = true, num_args = 1..)]
    command: Vec<OsString>,
}

#[derive(Args, Debug)]
struct RunOptions {
    /// Directory to compare before/after; cannot detect modifications outside this tree.
    #[arg(long, default_value = ".")]
    scope: PathBuf,
    /// Explicitly allow watching the entire root filesystem (expensive).
    #[arg(long)]
    allow_root_scope: bool,
    #[command(flatten)]
    common: Common,
}

#[derive(Args, Debug)]
struct SandboxOptions {
    /// Copy this directory into the isolated /work directory before execution.
    #[arg(long)]
    seed: Option<PathBuf>,
    /// Copy the final sandbox workspace to this directory (must not exist).
    #[arg(long)]
    export_workspace: Option<PathBuf>,
    #[command(flatten)]
    common: Common,
}

fn main() -> ExitCode {
    match execute(Cli::parse()) {
        Ok(code) => ExitCode::from(u8::try_from(code.clamp(0, 255)).unwrap_or(1)),
        Err(err) => {
            eprintln!("shelltrace: {err}");
            ExitCode::FAILURE
        }
    }
}

fn execute(cli: Cli) -> Result<i32, String> {
    match cli.action {
        Action::Run(opts) => {
            require_tool("strace")?;
            let scope = opts.scope.canonicalize().map_err(|e| format!("invalid --scope: {e}"))?;
            if scope == Path::new("/") && !opts.allow_root_scope {
                return Err("refusing to snapshot /; pass --allow-root-scope explicitly".into());
            }
            if !scope.is_dir() {
                return Err("--scope must name a directory".into());
            }
            run_trace("host", &scope, &opts.common, &opts.common.command, None)
        }
        Action::Sandbox(opts) => {
            require_tool("strace")?;
            require_tool("bwrap")?;
            let work = tempfile::tempdir().map_err(|e| format!("sandbox workspace: {e}"))?;
            if let Some(seed) = &opts.seed {
                sandbox::copy_tree(seed, work.path()).map_err(|e| format!("--seed: {e}"))?;
            }
            let mut wrapper = sandbox::wrapper(work.path());
            wrapper.extend(opts.common.command.iter().cloned());
            let result = run_trace("sandbox", work.path(), &opts.common, &wrapper, Some("/work"));
            if let Some(dst) = &opts.export_workspace {
                sandbox::export_tree(work.path(), dst)
                    .map_err(|e| format!("--export-workspace: {e}"))?;
                println!("Sandbox workspace exported to {}", dst.display());
            }
            result
        }
    }
}

fn require_tool(name: &str) -> Result<(), String> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    if std::env::split_paths(&path).any(|dir| dir.join(name).is_file()) {
        Ok(())
    } else {
        Err(format!("required executable '{name}' not found in PATH"))
    }
}

fn run_trace(
    mode: &str,
    scope: &Path,
    common: &Common,
    executed: &[OsString],
    display_scope: Option<&str>,
) -> Result<i32, String> {
    // Raw strace output is generated during execution, so keeping it inside the
    // observed directory would pollute the before/after diff.
    if mode == "host" {
        if let Some(keep_dir) = &common.keep_trace {
            let absolute = if keep_dir.is_absolute() {
                keep_dir.clone()
            } else {
                std::env::current_dir().map_err(|e| format!("current directory: {e}"))?.join(keep_dir)
            };
            if absolute.starts_with(scope) {
                return Err("--keep-trace must be outside --scope to avoid contaminating the filesystem diff".into());
            }
        }
    }
    let limit = common.max_hash_mib.saturating_mul(1024 * 1024);
    let before = snapshot(scope, limit).map_err(|e| format!("snapshot before: {e}"))?;
    let raw = match &common.keep_trace {
        Some(dir) => {
            fs::create_dir_all(dir).map_err(|e| format!("--keep-trace directory: {e}"))?;
            tempfile::Builder::new().prefix("shelltrace-").tempdir_in(dir)
                .map_err(|e| format!("trace directory: {e}"))?.keep()
        }
        None => tempfile::tempdir().map_err(|e| format!("trace directory: {e}"))?.keep(),
    };
    let trace_prefix = raw.join("trace");
    let started = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let timer = Instant::now();
    let status = Command::new("strace")
        .args(["-ff", "-qq", "-s", "512", "-yy", "-e", "trace=%file,%process,%network", "-o"])
        .arg(&trace_prefix)
        .arg("--")
        .args(executed)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("unable to start strace: {e}"))?;
    let elapsed_ms = u64::try_from(timer.elapsed().as_millis()).unwrap_or(u64::MAX);
    let after = snapshot(scope, limit).map_err(|e| format!("snapshot after: {e}"))?;
    let (changes, mut warnings) = compare(&before, &after);
    warnings.extend(before.warnings);
    warnings.extend(after.warnings);
    let (observations, trace_warnings) = trace::parse_dir(&raw, 20_000);
    warnings.extend(trace_warnings);
    if mode == "sandbox" {
        warnings.push("Bubblewrap reduces host-write exposure, but is NOT a hardened security boundary; the read-only host root remains visible, and namespace support varies".to_string());
        warnings.push("Sandbox snapshot covers only /work; tmpfs writes elsewhere are not included in the filesystem diff".into());
    } else {
        warnings.push("Host execution is NOT isolated: the command may modify files outside --scope or make network connections".into());
    }
    warnings.push("Strace syscall output describes observed calls/attempts, not a complete inventory of kernel effects or externally visible outcomes".into());
    if common.max_hash_mib == 0 {
        warnings.push("Content hashing disabled; same-size in-place edits may be missed".into());
    }
    let code = status.code().unwrap_or(128 + status.signal().unwrap_or(1));
    let report = Report {
        schema_version: 1,
        mode: mode.to_string(),
        started_at_unix: started,
        duration_ms: elapsed_ms,
        command: common.command.iter().map(|s| s.to_string_lossy().into_owned()).collect(),
        scope: display_scope.map(str::to_owned).unwrap_or_else(|| scope.display().to_string()),
        exit_code: code,
        filesystem_changes: changes,
        observations,
        warnings,
        raw_trace_path: common.keep_trace.as_ref().map(|_| raw.display().to_string()),
    };
    print_summary(&report);
    if let Some(path) = &common.json {
        let json = serde_json::to_vec_pretty(&report).map_err(|e| format!("JSON serialization: {e}"))?;
        fs::write(path, json).map_err(|e| format!("writing {}: {e}", path.display()))?;
        println!("JSON report: {}", path.display());
    }
    if common.keep_trace.is_none() {
        if let Err(err) = fs::remove_dir_all(&raw) {
            eprintln!("shelltrace: warning: failed to remove temporary trace {}: {err}", raw.display());
        }
    }
    Ok(code)
}

fn print_summary(report: &Report) {
    println!("\nShellTrace v{} | {} | exit={} | {} ms", env!("CARGO_PKG_VERSION"), report.mode, report.exit_code, report.duration_ms);
    println!("Scope: {}", report.scope);
    println!("Filesystem diff: {} changes", report.filesystem_changes.len());
    for change in report.filesystem_changes.iter().take(40) {
        println!("  {:<16} {} ({})", change.kind, change.path, change.evidence);
    }
    if report.filesystem_changes.len() > 40 {
        println!("  ... {} more in JSON", report.filesystem_changes.len() - 40);
    }
    println!(
        "Syscall observations: {} filesystem operations, {} process events, {} network events, {} sensitive accesses",
        report.observations.filesystem_operations.len(),
        report.observations.process_events.len(),
        report.observations.network_events.len(),
        report.observations.sensitive_accesses.len()
    );
    for event in report.observations.filesystem_operations.iter().take(8) {
        println!("  FILE pid={} {} {:?} -> {}", event.pid, event.syscall, event.paths, event.result);
    }
    for event in report.observations.process_events.iter().filter(|event| event.executable.is_some()).take(8) {
        println!("  PROCESS pid={} {} {:?} -> {}", event.pid, event.syscall, event.executable, event.result);
    }
    for event in report.observations.network_events.iter().take(8) {
        println!("  NETWORK pid={} {} {} -> {}", event.pid, event.syscall, event.address, event.result);
    }
    for event in report.observations.sensitive_accesses.iter().take(8) {
        println!("  SENSITIVE pid={} {} {} -> {}", event.pid, event.syscall, event.path, event.result);
    }
    for warning in report.warnings.iter().take(10) {
        eprintln!("  WARNING: {warning}");
    }
}

use std::os::unix::process::ExitStatusExt;
