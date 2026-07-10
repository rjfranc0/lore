use anyhow::Result;
use std::path::Path;

pub fn create(src: &Path, dst: &Path) -> Result<()> {
    #[cfg(unix)]
    std::os::unix::fs::symlink(src, dst)?;
    #[cfg(not(unix))]
    anyhow::bail!("symlinks not supported on this platform");
    Ok(())
}

/// Returns true if `path` is a symlink (even a broken one).
pub fn is_link(path: &Path) -> bool {
    path.symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
}

/// Returns true if the symlink target is a live directory (matches bash's `[[ -d ]]` semantics).
pub fn is_live(path: &Path) -> bool {
    path.is_dir()
}
