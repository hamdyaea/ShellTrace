use crate::report::FileChange;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{self, Read};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub file_type: String,
    pub size: u64,
    pub mode: u32,
    pub mtime_sec: i64,
    pub mtime_nsec: i64,
    pub digest: Option<String>,
    pub target: Option<String>,
}

#[derive(Default, Debug)]
pub struct Snapshot {
    pub entries: BTreeMap<String, Entry>,
    pub warnings: Vec<String>,
}

pub fn snapshot(root: &Path, hash_limit: u64) -> io::Result<Snapshot> {
    let mut out = Snapshot::default();
    for item in WalkDir::new(root).follow_links(false).sort_by_file_name() {
        let item = match item {
            Ok(item) => item,
            Err(err) => {
                out.warnings.push(format!("snapshot traversal: {err}"));
                continue;
            }
        };
        if item.path() == root {
            continue;
        }
        let rel = item.path().strip_prefix(root).map_err(io::Error::other)?;
        let path = rel.to_string_lossy().into_owned();
        let metadata = match fs::symlink_metadata(item.path()) {
            Ok(metadata) => metadata,
            Err(err) => {
                out.warnings.push(format!("cannot stat {path}: {err}"));
                continue;
            }
        };
        let file_type = metadata.file_type();
        let kind = if file_type.is_file() { "file" } else if file_type.is_dir() { "directory" } else if file_type.is_symlink() { "symlink" } else { "other" };
        let target = if file_type.is_symlink() {
            match fs::read_link(item.path()) {
                Ok(link) => Some(link.to_string_lossy().into_owned()),
                Err(err) => {
                    out.warnings.push(format!("cannot read symlink {path}: {err}"));
                    None
                }
            }
        } else {
            None
        };
        let digest = if file_type.is_file() && metadata.len() <= hash_limit {
            match hash_file(item.path()) {
                Ok(hash) => Some(hash),
                Err(err) => {
                    out.warnings.push(format!("cannot hash {path}: {err}"));
                    None
                }
            }
        } else {
            None
        };
        out.entries.insert(path, Entry {
            file_type: kind.to_string(),
            size: metadata.len(),
            mode: metadata.permissions().mode() & 0o7777,
            mtime_sec: metadata.mtime(),
            mtime_nsec: metadata.mtime_nsec(),
            digest,
            target,
        });
    }
    Ok(out)
}

fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 { break; }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn compare(before: &Snapshot, after: &Snapshot) -> (Vec<FileChange>, Vec<String>) {
    let keys: BTreeSet<_> = before.entries.keys().chain(after.entries.keys()).collect();
    let mut changes = Vec::new();
    let mut warnings = Vec::new();
    for key in keys {
        let lhs = before.entries.get(key);
        let rhs = after.entries.get(key);
        let result = match (lhs, rhs) {
            (None, Some(_)) => Some(("created", "path appeared")),
            (Some(_), None) => Some(("deleted", "path disappeared")),
            (Some(a), Some(b)) if a.file_type != b.file_type => Some(("type_changed", "file type changed")),
            (Some(a), Some(b)) if a.file_type == "directory" => {
                if a.mode != b.mode { Some(("metadata_changed", "directory mode changed")) } else { None }
            }
            (Some(a), Some(b)) if a.target != b.target => Some(("modified", "symlink target changed")),
            (Some(a), Some(b)) if a.size != b.size => Some(("modified", "size changed")),
            (Some(a), Some(b)) if a.digest.is_some() && b.digest.is_some() && a.digest != b.digest => Some(("modified", "SHA-256 changed")),
            (Some(a), Some(b)) if a.mode != b.mode => Some(("metadata_changed", "file mode changed")),
            (Some(a), Some(b)) if a.mtime_sec != b.mtime_sec || a.mtime_nsec != b.mtime_nsec => {
                if a.file_type == "file" && (a.digest.is_none() || b.digest.is_none()) {
                    warnings.push(format!("{key}: metadata changed but contents were not hashed (size limit or unreadable)"));
                    Some(("possibly_modified", "mtime changed; contents unverified"))
                } else {
                    Some(("metadata_changed", "mtime changed; identical hashed content"))
                }
            }
            _ => None,
        };
        if let Some((kind, evidence)) = result {
            changes.push(FileChange { path: key.clone(), kind: kind.to_owned(), evidence: evidence.to_owned() });
        }
    }
    (changes, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_modifies_and_deletes() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("old"), "abc").unwrap();
        fs::write(dir.path().join("remove"), "gone").unwrap();
        let before = snapshot(dir.path(), 1_000).unwrap();
        fs::write(dir.path().join("old"), "xyz").unwrap();
        fs::remove_file(dir.path().join("remove")).unwrap();
        fs::write(dir.path().join("new"), "hello").unwrap();
        let after = snapshot(dir.path(), 1_000).unwrap();
        let (changes, _) = compare(&before, &after);
        assert!(changes.iter().any(|c| c.kind == "modified" && c.path == "old"));
        assert!(changes.iter().any(|c| c.kind == "deleted" && c.path == "remove"));
        assert!(changes.iter().any(|c| c.kind == "created" && c.path == "new"));
    }

    #[test]
    fn mode_change_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file");
        fs::write(&path, "text").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        let before = snapshot(dir.path(), 100).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let after = snapshot(dir.path(), 100).unwrap();
        assert!(compare(&before, &after).0.iter().any(|c| c.kind == "metadata_changed"));
    }

    #[test]
    fn oversized_file_mtime_is_uncertain() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("file"), "oversized").unwrap();
        let before = snapshot(dir.path(), 0).unwrap();
        let after = snapshot(dir.path(), 0).unwrap();
        assert!(compare(&before, &after).0.is_empty());
    }
}
