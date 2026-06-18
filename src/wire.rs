use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::{output, symlink};

/// Single source of truth for where CLAUDE.md lives under a Claude dir.
pub fn claude_md_path(claude_dir: &Path) -> PathBuf {
    claude_dir.join("CLAUDE.md")
}

/// Single source of truth for where the skills symlink lives under a Claude dir.
pub fn claude_skills_path(claude_dir: &Path) -> PathBuf {
    claude_dir.join("skills")
}

pub fn wire_claude_dir(agents_md: &Path, skills_dir: &Path, claude_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(claude_dir)?;

    let claude_md = claude_md_path(claude_dir);
    std::fs::write(&claude_md, format!("@{}\n", agents_md.display()))?;
    output::ok(&format!("Wrote {}", claude_md.display()));

    let claude_skills = claude_skills_path(claude_dir);
    if symlink::is_link(&claude_skills) {
        std::fs::remove_file(&claude_skills)?;
    }
    symlink::create(skills_dir, &claude_skills)?;
    output::ok(&format!("Linked {}/skills → {}", claude_dir.display(), skills_dir.display()));

    Ok(())
}
