#![cfg(target_os = "linux")]

use std::fs;
use std::process::Command;

#[test]
fn run_writes_json_with_a_confirmed_file_creation() {
    // Some local development machines have neither strace nor ptrace enabled.
    if Command::new("strace").arg("-V").output().is_err() {
        eprintln!("skipping integration test: strace is unavailable");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let scope = dir.path().join("scope");
    fs::create_dir(&scope).unwrap();
    let report = dir.path().join("report.json");
    let output = Command::new(env!("CARGO_BIN_EXE_shelltrace"))
        .arg("run")
        .arg("--scope").arg(&scope)
        .arg("--json").arg(&report)
        .arg("--")
        .arg("/bin/sh")
        .arg("-c")
        .arg("printf test > created.txt")
        .current_dir(&scope)
        .output().unwrap();
    assert!(output.status.success(), "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    let data = fs::read_to_string(report).unwrap();
    let json: serde_json::Value = serde_json::from_str(&data).unwrap();
    assert_eq!(json["schema_version"], 1);
    let changes = json["filesystem_changes"].as_array().unwrap();
    assert!(changes.iter().any(|c| c["path"] == "created.txt" && c["kind"] == "created"));
}

#[test]
fn sandbox_creates_file_in_private_workspace() {
    if Command::new("strace").arg("-V").output().is_err() ||
       Command::new("bwrap").arg("--version").output().is_err() {
        eprintln!("skipping sandbox test: required tools not installed");
        return;
    }

    // First test namespace availability independently of ShellTrace's
    // mount layout. A broken workspace mount must not become a silent skip.
    let probe = Command::new("bwrap")
        .args(["--unshare-all", "--ro-bind", "/", "/", "--", "/bin/true"])
        .output().unwrap();
    if !probe.status.success() {
        eprintln!("skipping sandbox test: user namespaces unavailable: {}",
            String::from_utf8_lossy(&probe.stderr));
        return;
    }

    let host = tempfile::tempdir().unwrap();
    let report = host.path().join("report.json");
    let output = Command::new(env!("CARGO_BIN_EXE_shelltrace"))
        .arg("sandbox")
        .arg("--json").arg(&report)
        .arg("--")
        .arg("/bin/sh")
        .arg("-c")
        .arg("echo test > regression-workspace.txt")
        .current_dir(host.path())
        .output().unwrap();
    assert!(output.status.success(), "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));

    let json: serde_json::Value = serde_json::from_slice(&fs::read(&report).unwrap()).unwrap();
    assert_eq!(json["mode"], "sandbox");
    assert_eq!(json["exit_code"], 0);
    assert_eq!(json["scope"], "/tmp/shelltrace-work");
    assert!(json["filesystem_changes"].as_array().unwrap()
        .iter().any(|c| c["path"] == "regression-workspace.txt" && c["kind"] == "created"));
    assert!(!host.path().join("regression-workspace.txt").exists(),
        "sandbox workspace leaked into the host current directory");
}
