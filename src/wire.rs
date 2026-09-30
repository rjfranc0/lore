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

/// Single source of truth for where subagents live under a Claude dir.
pub fn claude_agents_path(claude_dir: &Path) -> PathBuf {
    claude_dir.join("agents")
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
/// See [`relink_into`] for the create-if-absent and collision rules.
pub fn relink_skill(skills_dir: &Path, claude_dir: &Path, name: &str) -> Result<()> {
    relink_into(skills_dir, &claude_skills_path(claude_dir), name)
}

/// True when `claude_dir/agents` is a symlink — the user's own setup, which
/// agent wiring, fan-out and scoped installs never write or delete through
/// (it may alias the shared pool, another account, or an unrelated directory).
pub fn agents_dir_is_symlink(claude_dir: &Path) -> bool {
    symlink::is_link(&claude_agents_path(claude_dir))
}

/// Re-links one shared subagent (`subagents_dir/<file_name>`) into
/// `claude_dir/agents/<file_name>`. `file_name` keeps its `.md` extension.
/// Warns and skips when `agents/` is a symlink instead of writing through it.
pub fn relink_agent(subagents_dir: &Path, claude_dir: &Path, file_name: &str) -> Result<()> {
    if agents_dir_is_symlink(claude_dir) {
        warn_symlinked_agents_dir(claude_dir, "skipping re-link");
        return Ok(());
    }
    relink_into(subagents_dir, &claude_agents_path(claude_dir), file_name)
}

fn warn_symlinked_agents_dir(claude_dir: &Path, consequence: &str) {
    output::warn(&format!(
        "{} is a symlink — leaving it untouched, {consequence}",
        claude_agents_path(claude_dir).display()
    ));
}

/// Create-if-absent: skips silently when a link already exists at
/// `account_subdir/<name>`, so a caller can call this per-entry (`install`) or
/// in a loop over all shared entries (`wire_claude_skills`) without ever
/// overwriting an existing link. If a non-symlink entry already occupies the
/// target (e.g. manual tampering, or a hand-written agent file), warns and
/// skips rather than failing the whole fan-out over one account's collision.
fn relink_into(shared_dir: &Path, account_subdir: &Path, name: &str) -> Result<()> {
    std::fs::create_dir_all(account_subdir)?;
    let link = account_subdir.join(name);
    if symlink::is_link(&link) {
        return Ok(());
    }
    if link.exists() {
        output::warn(&format!(
            "{name} exists in {} and is not a symlink — skipping re-link",
            account_subdir.display()
        ));
        return Ok(());
    }
    symlink::create(&shared_dir.join(name), &link)?;
    Ok(())
}

/// Removes the account's re-link for `name` from `claude_dir/skills`, but
/// only when it actually points at `skills_dir/<name>` — an account-scoped
/// install of the same name that points elsewhere is left untouched rather
/// than silently destroyed. Silent no-op when no link is present.
pub fn unlink_account_skill(claude_dir: &Path, skills_dir: &Path, name: &str) -> Result<()> {
    unlink_from(&claude_skills_path(claude_dir), skills_dir, name)
}

/// Agent counterpart of [`unlink_account_skill`]; `file_name` keeps its `.md`.
/// Warns and skips when `agents/` is a symlink instead of deleting through it.
pub fn unlink_account_agent(
    claude_dir: &Path,
    subagents_dir: &Path,
    file_name: &str,
) -> Result<()> {
    if agents_dir_is_symlink(claude_dir) {
        warn_symlinked_agents_dir(claude_dir, "skipping unlink");
        return Ok(());
    }
    unlink_from(&claude_agents_path(claude_dir), subagents_dir, file_name)
}

fn unlink_from(account_subdir: &Path, shared_dir: &Path, name: &str) -> Result<()> {
    let link = account_subdir.join(name);
    if !symlink::is_link(&link) {
        return Ok(());
    }
    let expected = shared_dir.join(name);
    match std::fs::read_link(&link) {
        Ok(target) if target == expected => std::fs::remove_file(&link)?,
        Ok(_) => output::warn(&format!(
            "{name} in {} points elsewhere (account-scoped install) — leaving it in place",
            account_subdir.display()
        )),
        Err(_) => {}
    }
    Ok(())
}

/// Ensures `claude_dir/agents` is a real directory holding a re-link for every
/// entry in `subagents_dir`. Unlike skills there is no legacy symlink model to
/// migrate, so a symlinked `agents/` is the user's own setup: warn and leave it
/// alone rather than write through it. Existing entries always survive.
pub fn wire_claude_agents(subagents_dir: &Path, claude_dir: &Path) -> Result<()> {
    if agents_dir_is_symlink(claude_dir) {
        warn_symlinked_agents_dir(claude_dir, "agents not wired");
        return Ok(());
    }
    let claude_agents = claude_agents_path(claude_dir);
    std::fs::create_dir_all(&claude_agents)?;

    if subagents_dir.is_dir() {
        for entry in std::fs::read_dir(subagents_dir)? {
            let entry = entry?;
            relink_agent(
                subagents_dir,
                claude_dir,
                &entry.file_name().to_string_lossy(),
            )?;
        }
    }

    output::ok(&format!(
        "Wired {} as re-links from {}",
        claude_agents.display(),
        subagents_dir.display()
    ));
    Ok(())
}

pub fn wire_claude_dir(
    agents_md: &Path,
    skills_dir: &Path,
    subagents_dir: &Path,
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
    wire_claude_agents(subagents_dir, claude_dir)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shared_agent(tmp: &tempfile::TempDir, name: &str) -> PathBuf {
        let subagents_dir = tmp.path().join("shared-agents");
        std::fs::create_dir_all(&subagents_dir).unwrap();
        std::fs::write(subagents_dir.join(name), "x").unwrap();
        subagents_dir
    }

    #[test]
    fn relink_agent_creates_link_when_absent_and_skips_when_present() {
        let tmp = tempfile::tempdir().unwrap();
        let subagents_dir = shared_agent(&tmp, "rev.md");
        let claude_dir = tmp.path().join("claude");

        relink_agent(&subagents_dir, &claude_dir, "rev.md").unwrap();
        let link = claude_agents_path(&claude_dir).join("rev.md");
        assert_eq!(
            std::fs::read_link(&link).unwrap(),
            subagents_dir.join("rev.md")
        );

        // Re-pointing the link elsewhere proves a second call never overwrites it.
        let other = tmp.path().join("other.md");
        std::fs::write(&other, "y").unwrap();
        std::fs::remove_file(&link).unwrap();
        symlink::create(&other, &link).unwrap();
        relink_agent(&subagents_dir, &claude_dir, "rev.md").unwrap();
        assert_eq!(std::fs::read_link(&link).unwrap(), other);
    }

    #[test]
    fn relink_agent_skips_real_file_in_the_way() {
        let tmp = tempfile::tempdir().unwrap();
        let subagents_dir = shared_agent(&tmp, "mine.md");
        let claude_dir = tmp.path().join("claude");
        let agents = claude_agents_path(&claude_dir);
        std::fs::create_dir_all(&agents).unwrap();
        std::fs::write(agents.join("mine.md"), "hand-written").unwrap();

        relink_agent(&subagents_dir, &claude_dir, "mine.md").unwrap();

        let link = agents.join("mine.md");
        assert!(!symlink::is_link(&link));
        assert_eq!(std::fs::read_to_string(&link).unwrap(), "hand-written");
    }

    #[test]
    fn unlink_account_agent_removes_only_links_to_the_shared_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let subagents_dir = shared_agent(&tmp, "rev.md");
        let claude_dir = tmp.path().join("claude");

        relink_agent(&subagents_dir, &claude_dir, "rev.md").unwrap();
        unlink_account_agent(&claude_dir, &subagents_dir, "rev.md").unwrap();
        let link = claude_agents_path(&claude_dir).join("rev.md");
        assert!(!symlink::is_link(&link));

        let scoped_src = tmp.path().join("scoped.md");
        std::fs::write(&scoped_src, "y").unwrap();
        symlink::create(&scoped_src, &link).unwrap();
        unlink_account_agent(&claude_dir, &subagents_dir, "rev.md").unwrap();
        assert_eq!(std::fs::read_link(&link).unwrap(), scoped_src);
    }

    #[test]
    fn wire_claude_agents_leaves_symlinked_agents_dir_untouched() {
        let tmp = tempfile::tempdir().unwrap();
        let subagents_dir = shared_agent(&tmp, "rev.md");
        let claude_dir = tmp.path().join("claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        let user_dir = tmp.path().join("user-agents");
        std::fs::create_dir_all(&user_dir).unwrap();
        symlink::create(&user_dir, &claude_agents_path(&claude_dir)).unwrap();

        wire_claude_agents(&subagents_dir, &claude_dir).unwrap();

        assert!(symlink::is_link(&claude_agents_path(&claude_dir)));
        assert_eq!(std::fs::read_dir(&user_dir).unwrap().count(), 0);
    }

    #[test]
    fn wire_claude_agents_relinks_shared_entries_and_keeps_existing_ones() {
        let tmp = tempfile::tempdir().unwrap();
        let subagents_dir = shared_agent(&tmp, "rev.md");
        let claude_dir = tmp.path().join("claude");
        let agents = claude_agents_path(&claude_dir);
        std::fs::create_dir_all(&agents).unwrap();
        std::fs::write(agents.join("mine.md"), "hand-written").unwrap();

        wire_claude_agents(&subagents_dir, &claude_dir).unwrap();

        assert!(symlink::is_link(&agents.join("rev.md")));
        assert_eq!(
            std::fs::read_to_string(agents.join("mine.md")).unwrap(),
            "hand-written"
        );
    }

    #[test]
    fn relink_agent_skips_symlinked_agents_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let subagents_dir = shared_agent(&tmp, "rev.md");
        let claude_dir = tmp.path().join("claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        let user_dir = tmp.path().join("user-agents");
        std::fs::create_dir_all(&user_dir).unwrap();
        symlink::create(&user_dir, &claude_agents_path(&claude_dir)).unwrap();

        relink_agent(&subagents_dir, &claude_dir, "rev.md").unwrap();

        assert_eq!(std::fs::read_dir(&user_dir).unwrap().count(), 0);
    }

    #[test]
    fn unlink_account_agent_skips_symlinked_agents_dir_aliasing_the_shared_pool() {
        let tmp = tempfile::tempdir().unwrap();
        let subagents_dir = shared_agent(&tmp, "rev.md");
        let claude_dir = tmp.path().join("claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        symlink::create(&subagents_dir, &claude_agents_path(&claude_dir)).unwrap();
        let shared_entry = subagents_dir.join("rev.md");
        std::fs::remove_file(&shared_entry).unwrap();
        let src = tmp.path().join("src.md");
        std::fs::write(&src, "x").unwrap();
        symlink::create(&src, &shared_entry).unwrap();

        unlink_account_agent(&claude_dir, &subagents_dir, "rev.md").unwrap();

        assert!(symlink::is_link(&shared_entry));
    }
}
