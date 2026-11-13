use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// The sandbox's writable mount point. A child of private /tmp avoids
/// creating /work under an otherwise read-only host-root bind mount.
pub const SANDBOX_WORKDIR: &str = "/tmp/shelltrace-work";

pub fn wrapper(workspace: &Path) -> Vec<OsString> {
    // The host root is read-only. /tmp exists on normal Linux systems,
    // so mounting a private tmpfs on it gives bwrap a writable parent
    // where it can create the destination for the workspace bind.
    // Do not create /work directly under the read-only root (EROFS).
    let arguments = [
        "bwrap", "--die-with-parent", "--new-session", "--unshare-all",
        "--ro-bind", "/", "/", "--dev", "/dev", "--proc", "/proc",
        "--tmpfs", "/tmp", "--bind",
    ];
    let mut args: Vec<OsString> = arguments.iter().map(|s| OsString::from(*s)).collect();
    args.push(workspace.as_os_str().to_os_string());
    args.push(OsString::from(SANDBOX_WORKDIR));
    args.extend([
        OsString::from("--tmpfs"), OsString::from("/var/tmp"),
        OsString::from("--tmpfs"), OsString::from("/run"),
        OsString::from("--tmpfs"), OsString::from("/home"),
        OsString::from("--tmpfs"), OsString::from("/root"),
        OsString::from("--tmpfs"), OsString::from("/mnt"),
        OsString::from("--tmpfs"), OsString::from("/media"),
        OsString::from("--chdir"), OsString::from(SANDBOX_WORKDIR),
        OsString::from("--clearenv"),
        OsString::from("--setenv"), OsString::from("HOME"), OsString::from(SANDBOX_WORKDIR),
        OsString::from("--setenv"), OsString::from("PATH"), OsString::from("/usr/local/bin:/usr/bin:/bin"),
        OsString::from("--"),
    ]);
    args
}

pub fn copy_tree(source: &Path, target: &Path) -> io::Result<()> {
    let source = source.canonicalize()?;
    if !source.is_dir() { return Err(io::Error::new(io::ErrorKind::InvalidInput, "source is not a directory")); }
    if target.starts_with(&source) { return Err(io::Error::new(io::ErrorKind::InvalidInput, "destination is inside source")); }
    for item in WalkDir::new(&source).follow_links(false) {
        let item = item.map_err(io::Error::other)?;
        let rel = item.path().strip_prefix(&source).map_err(io::Error::other)?;
        if rel.as_os_str().is_empty() { continue; }
        let destination: PathBuf = target.join(rel);
        let metadata = fs::symlink_metadata(item.path())?;
        if metadata.is_dir() {
            fs::create_dir_all(&destination)?;
            // Do not make a directory read-only before its children are copied.
            // Directory permission changes in a seed are not security controls.
        } else if metadata.file_type().is_symlink() {
            symlink(fs::read_link(item.path())?, &destination)?;
        } else if metadata.is_file() {
            let mut reader = fs::File::open(item.path())?;
            let mut writer = OpenOptions::new().write(true).create_new(true).open(&destination)?;
            io::copy(&mut reader, &mut writer)?;
            writer.set_permissions(fs::Permissions::from_mode(metadata.permissions().mode()))?;
        } else {
            return Err(io::Error::new(io::ErrorKind::Unsupported, format!("unsupported special file: {}", item.path().display())));
        }
    }
    Ok(())
}

pub fn export_tree(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir(destination)?;
    // On failure the partial export remains available for inspection.
    copy_tree(source, destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copying_seed_preserves_files_and_symlinks() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        fs::write(src.path().join("file.txt"), "hello").unwrap();
        symlink("file.txt", src.path().join("alias")).unwrap();
        copy_tree(src.path(), dst.path()).unwrap();
        assert_eq!(fs::read_to_string(dst.path().join("file.txt")).unwrap(), "hello");
        assert_eq!(fs::read_link(dst.path().join("alias")).unwrap(), Path::new("file.txt"));
    }

    #[test]
    fn wrapper_disables_network_and_has_read_only_root() {
        let args = wrapper(Path::new("/tmp/test"));
        assert!(args.contains(&OsString::from("--unshare-all")));
        assert!(args.contains(&OsString::from("--ro-bind")));
        let bind = args.iter().position(|a| a == "--bind").unwrap();
        let tmpfs = args.iter().position(|a| a == "--tmpfs").unwrap();
        assert!(tmpfs < bind, "private /tmp must exist before the workspace bind");
        assert_eq!(args.get(bind + 2), Some(&OsString::from(SANDBOX_WORKDIR)));
        assert!(!args.contains(&OsString::from("/work")));
    }
}
