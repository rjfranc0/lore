use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::{agents_md::AgentsMd, output, symlink};

/// Single source of truth for where CLAUDE.md lives under a Claude dir.
pub fn claude_md_path(claude_dir: &Path) -> PathBuf {
    claude_dir.join("CLAUDE.md")
}

/// Single source of truth for where LORE.md lives under a Claude dir.
pub fn lore_md_path(claude_dir: &Path) -> PathBuf {
    claude_dir.join("LORE.md")
}

/// Single source of truth for where the skills symlink lives under a Claude dir.
pub fn claude_skills_path(claude_dir: &Path) -> PathBuf {
    claude_dir.join("skills")
}

/// Single source of truth for where account-scoped behaviors live under a Claude dir.
pub fn claude_behaviors_path(claude_dir: &Path) -> PathBuf {
    claude_dir.join("behaviors")
}

/// Creates or updates LORE.md so its header imports `agents_md`. LORE.md is
/// fully lore-owned, so the header is unconditionally overwritten rather than
/// checked first — that's what keeps this idempotent without a separate
/// "already correct" branch. Behavior blocks already registered in LORE.md
/// (via `behavior add --account`) are preserved.
pub fn wire_lore_md(agents_md: &Path, claude_dir: &Path) -> Result<PathBuf> {
    let lore_md = lore_md_path(claude_dir);
    let mut md = if lore_md.exists() {
        AgentsMd::load(&lore_md).unwrap_or_else(|e| {
            output::warn(&format!(
                "{} is unreadable ({e}) — recreating it",
                lore_md.display()
            ));
            AgentsMd::parse("")
        })
    } else {
        AgentsMd::parse("")
    };
    md.header = format!("@{}\n", agents_md.display());
    md.save(&lore_md)?;
    Ok(lore_md)
}

/// Surgically wires CLAUDE.md to import LORE.md — CLAUDE.md is never fully
/// overwritten. `agents_md` is only used to recognize the legacy
/// pre-LORE.md direct-import line; it is never written into CLAUDE.md here.
pub fn wire_claude_md(
    claude_dir: &Path,
    agents_md: &Path,
    migration_behaviors_dir: &Path,
    migration_register_md: &Path,
) -> Result<()> {
    let claude_md = claude_md_path(claude_dir);
    let lore_md = lore_md_path(claude_dir);
    let lore_line = format!("@{}", lore_md.display());

    if symlink::is_link(&claude_md) {
        std::fs::remove_file(&claude_md)?;
    } else if claude_md.is_dir() {
        std::fs::remove_dir_all(&claude_md)?;
    }

    let content = if claude_md.exists() {
        match std::fs::read_to_string(&claude_md) {
            Ok(c) => Some(c),
            Err(e) => {
                output::warn(&format!(
                    "{} is unreadable ({e}) — replacing with a fresh LORE.md import",
                    claude_md.display()
                ));
                None
            }
        }
    } else {
        None
    };

    if let Some(content) = &content {
        if content.lines().any(|l| l.trim() == lore_line) {
            return Ok(());
        }

        let agents_line = format!("@{}", agents_md.display());
        if let Some(pos) = content.lines().position(|l| l.trim() == agents_line) {
            let updated = content
                .lines()
                .enumerate()
                .map(|(i, l)| if i == pos { lore_line.as_str() } else { l })
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            std::fs::write(&claude_md, updated)?;
            output::ok(&format!(
                "Updated {} to import LORE.md",
                claude_md.display()
            ));
            return Ok(());
        }

        if !content.trim().is_empty() {
            return migrate_claude_md(
                content,
                &claude_md,
                &lore_line,
                migration_behaviors_dir,
                migration_register_md,
            );
        }
    }

    std::fs::write(&claude_md, format!("{lore_line}\n"))?;
    output::ok(&format!("Wrote {}", claude_md.display()));
    Ok(())
}

/// Copies pre-existing CLAUDE.md content into a `from-claude` behavior,
/// registers it, then appends the LORE.md import to the *original* content
/// — nothing is deleted from the live file.
fn migrate_claude_md(
    old_content: &str,
    claude_md: &Path,
    lore_line: &str,
    behaviors_dir: &Path,
    register_md: &Path,
) -> Result<()> {
    let rules = behaviors_dir.join("from-claude").join("RULES.md");
    std::fs::create_dir_all(rules.parent().unwrap())?;
    std::fs::write(&rules, old_content)?;

    let mut md = AgentsMd::load(register_md)?;
    if !md.contains_name("from-claude") {
        md.add("from-claude".into(), rules.clone());
        md.save(register_md)?;
    }

    let mut updated = old_content.trim_end().to_string();
    updated.push_str("\n\n");
    updated.push_str(lore_line);
    updated.push('\n');
    std::fs::write(claude_md, updated)?;

    output::ok(&format!(
        "Migrated {} → {}",
        claude_md.display(),
        rules.display()
    ));
    for line in old_content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('@') {
            output::note(&format!(
                "Found an existing import, left untouched: {trimmed}"
            ));
        }
    }
    output::note(&format!(
        "{} is no longer fully managed — add rules via behaviors instead of hand-editing it.",
        claude_md.display()
    ));

    Ok(())
}

/// Ensures `claude_dir/skills` is a real directory holding a re-link for every
/// entry in `skills_dir`. Removes a stale single-symlink (the legacy model)
/// first, but never wipes the directory afterward — account-specific symlinks
/// already present must survive every call. Idempotent: existing re-links and
/// account-specific symlinks are left untouched.
pub fn wire_claude_skills(skills_dir: &Path, claude_dir: &Path) -> Result<()> {
    let claude_skills = claude_skills_path(claude_dir);
    if symlink::is_link(&claude_skills) {
        std::fs::remove_file(&claude_skills)?;
    }
    std::fs::create_dir_all(&claude_skills)?;

    if skills_dir.is_dir() {
        for entry in std::fs::read_dir(skills_dir)? {
            let entry = entry?;
            relink_skill(skills_dir, claude_dir, &entry.file_name().to_string_lossy())?;
        }
    }

    output::ok(&format!(
        "Wired {} as re-links from {}",
        claude_skills.display(),
        skills_dir.display()
    ));
    Ok(())
}

/// Re-links one shared skill (`skills_dir/<name>`) into `claude_dir/skills/<name>`.
/// Create-if-absent: skips silently when a link already exists there, so a
/// caller can call this per-skill (`install`) or in a loop over all shared
/// skills (`wire_claude_skills`) without ever overwriting an existing link.
/// If a non-symlink entry already occupies the target (e.g. manual tampering,
/// since lore itself never places one there post-init), warns and skips
/// rather than failing the whole fan-out over one account's collision.
pub fn relink_skill(skills_dir: &Path, claude_dir: &Path, name: &str) -> Result<()> {
    let claude_skills = claude_skills_path(claude_dir);
    std::fs::create_dir_all(&claude_skills)?;
    let link = claude_skills.join(name);
    if symlink::is_link(&link) {
        return Ok(());
    }
    if link.exists() {
        output::warn(&format!(
            "{name} exists in {} and is not a symlink — skipping re-link",
            claude_skills.display()
        ));
        return Ok(());
    }
    symlink::create(&skills_dir.join(name), &link)?;
    Ok(())
}

/// Removes the account's re-link for `name` from `claude_dir/skills`, but
/// only when it actually points at `skills_dir/<name>` — an account-scoped
/// install of the same name that points elsewhere is left untouched rather
/// than silently destroyed. Silent no-op when no link is present.
pub fn unlink_account_skill(claude_dir: &Path, skills_dir: &Path, name: &str) -> Result<()> {
    let link = claude_skills_path(claude_dir).join(name);
    if !symlink::is_link(&link) {
        return Ok(());
    }
    let expected = skills_dir.join(name);
    match std::fs::read_link(&link) {
        Ok(target) if target == expected => std::fs::remove_file(&link)?,
        Ok(_) => output::warn(&format!(
            "{name} in {} points elsewhere (account-scoped install) — leaving it in place",
            claude_skills_path(claude_dir).display()
        )),
        Err(_) => {}
    }
    Ok(())
}

pub fn wire_claude_dir(
    agents_md: &Path,
    skills_dir: &Path,
    claude_dir: &Path,
    migration_behaviors_dir: &Path,
    migration_register_md: &Path,
) -> Result<()> {
    std::fs::create_dir_all(claude_dir)?;

    // LORE.md must exist before wire_claude_md runs: CLAUDE.md's new content
    // names it, and a Case-3 migration registers into this same file.
    wire_lore_md(agents_md, claude_dir)?;
    wire_claude_md(
        claude_dir,
        agents_md,
        migration_behaviors_dir,
        migration_register_md,
    )?;
    wire_claude_skills(skills_dir, claude_dir)?;

    Ok(())
}
