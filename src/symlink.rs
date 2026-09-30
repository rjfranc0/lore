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

/// Returns true if the symlink target is a live regular file (follows the link).
pub fn is_live_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_live_file_true_for_link_to_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("a.md");
        std::fs::write(&file, "x").unwrap();
        let link = tmp.path().join("link.md");
        create(&file, &link).unwrap();
        assert!(is_live_file(&link));
    }

    #[test]
    fn is_live_file_false_for_missing_target() {
        let tmp = tempfile::tempdir().unwrap();
        let link = tmp.path().join("link.md");
        create(&tmp.path().join("gone.md"), &link).unwrap();
        assert!(!is_live_file(&link));
    }

    #[test]
    fn is_live_file_false_for_link_to_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("d");
        std::fs::create_dir_all(&dir).unwrap();
        let link = tmp.path().join("link.md");
        create(&dir, &link).unwrap();
        assert!(!is_live_file(&link));
    }
}
