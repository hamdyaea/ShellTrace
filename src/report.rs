use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub mode: String,
    pub started_at_unix: u64,
    pub duration_ms: u64,
    pub command: Vec<String>,
    pub scope: String,
    pub exit_code: i32,
    pub filesystem_changes: Vec<FileChange>,
    pub observations: Observations,
    pub warnings: Vec<String>,
    pub raw_trace_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FileState {
    pub file_type: String,
    pub size_bytes: u64,
    /// POSIX permission bits encoded as an integer. Render as octal for humans.
    pub mode: u32,
    pub sha256: Option<String>,
    pub symlink_target: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    pub kind: String,
    pub evidence: String,
    /// Observed state before execution, not an archived copy of the file.
    pub before: Option<FileState>,
    /// Observed state after execution, not an archived copy of the file.
    pub after: Option<FileState>,
}

#[derive(Default, Debug, Serialize)]
pub struct Observations {
    pub filesystem_operations: Vec<FileOperation>,
    pub process_events: Vec<ProcessEvent>,
    pub network_events: Vec<NetworkEvent>,
    pub sensitive_accesses: Vec<SensitiveAccess>,
    pub truncated: bool,
}

/// Attribution is conservative. "unknown" does not imply that an event is safe.
pub const ORIGIN_COMMAND: &str = "command";
pub const ORIGIN_SANDBOX_SETUP: &str = "sandbox_setup";
pub const ORIGIN_UNKNOWN: &str = "unknown";

#[derive(Debug, Serialize)]
pub struct FileOperation {
    pub pid: u32,
    pub syscall: String,
    pub paths: Vec<String>,
    pub result: String,
    pub origin: String,
}

#[derive(Debug, Serialize)]
pub struct ProcessEvent {
    pub pid: u32,
    pub syscall: String,
    pub executable: Option<String>,
    pub result: String,
    pub origin: String,
}

#[derive(Debug, Serialize)]
pub struct NetworkEvent {
    pub pid: u32,
    pub syscall: String,
    pub address: String,
    pub result: String,
    pub origin: String,
    /// internet, local_ipc, kernel_ipc or unknown; AF_NETLINK is NOT Internet traffic.
    pub category: String,
    pub succeeded: bool,
}

#[derive(Debug, Serialize)]
pub struct SensitiveAccess {
    pub pid: u32,
    pub syscall: String,
    pub path: String,
    pub result: String,
    pub origin: String,
}
