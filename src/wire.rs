use std::path::Path;

use anyhow::Result;

use crate::{output, symlink};

pub fn wire_claude_dir(agents_md: &Path, skills_dir: &Path, claude_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(claude_dir)?;

    let claude_md = claude_dir.join("CLAUDE.md");
    std::fs::write(&claude_md, format!("@{}\n", agents_md.display()))?;
    output::ok(&format!("Wrote {}", claude_md.display()));

    let claude_skills = claude_dir.join("skills");
    if symlink::is_link(&claude_skills) {
        std::fs::remove_file(&claude_skills)?;
    }
    symlink::create(skills_dir, &claude_skills)?;
    output::ok(&format!("Linked {}/skills → {}", claude_dir.display(), skills_dir.display()));

    Ok(())
}
