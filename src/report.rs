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

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    pub kind: String,
    pub evidence: String,
}

#[derive(Default, Debug, Serialize)]
pub struct Observations {
    pub filesystem_operations: Vec<FileOperation>,
    pub process_events: Vec<ProcessEvent>,
    pub network_events: Vec<NetworkEvent>,
    pub sensitive_accesses: Vec<SensitiveAccess>,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
pub struct FileOperation {
    pub pid: u32,
    pub syscall: String,
    pub paths: Vec<String>,
    pub result: String,
}

#[derive(Debug, Serialize)]
pub struct ProcessEvent {
    pub pid: u32,
    pub syscall: String,
    pub executable: Option<String>,
    pub result: String,
}

#[derive(Debug, Serialize)]
pub struct NetworkEvent {
    pub pid: u32,
    pub syscall: String,
    pub address: String,
    pub result: String,
}

#[derive(Debug, Serialize)]
pub struct SensitiveAccess {
    pub pid: u32,
    pub syscall: String,
    pub path: String,
    pub result: String,
}
