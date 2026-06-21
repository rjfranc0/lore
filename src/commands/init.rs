use anyhow::Result;

use crate::{
    agents_md::{AgentsMd, behavior_entry},
    config::LoreConfig,
    output,
    paths::Paths,
    symlink, wire,
};

const AGENTS_MD_HEADER: &str = "\
<!-- managed by lore — do not edit -->
<!-- skills auto-loaded from ~/.agents/skills/ -->
";

pub fn run(account: Option<String>) -> Result<()> {
    if let Some(name) = &account {
        if name.is_empty() {
            anyhow::bail!("invalid account name: must not be empty");
        }
        if let Some(bad) = name
            .chars()
            .find(|c| !c.is_ascii_alphanumeric() && *c != '-')
        {
            anyhow::bail!(
                "invalid account name '{name}': only alphanumeric characters and hyphens are allowed (found '{bad}')"
            );
        }
    }

    let config_path = LoreConfig::config_path();
    let mut config = LoreConfig::load_or_default(&config_path)?;
    let p = Paths::from_config(&config);

    let account_name = account.clone().unwrap_or_else(|| "default".to_string());
    let claude_dir = if account_name == "default" {
        // Explicit `--account default` is a no-op alias for omitting the flag —
        // both must resolve through the same registry entry, never a second,
        // untracked `~/.claude-default/`.
        config.account_path("default").unwrap_or_else(|| {
            dirs::home_dir()
                .expect("cannot determine home directory")
                .join(".claude")
        })
    } else {
        dirs::home_dir()
            .expect("cannot determine home directory")
            .join(format!(".claude-{account_name}"))
    };

    std::fs::create_dir_all(&p.skills_dir)?;
    std::fs::create_dir_all(&p.behaviors_dir)?;

    let claude_md = wire::claude_md_path(&claude_dir);
    let claude_skills = wire::claude_skills_path(&claude_dir);

    // ── AGENTS.MD ─────────────────────────────────────────────────────────────

    if !p.agents_md.exists() {
        let should_migrate = if claude_md.exists() {
            let content = std::fs::read_to_string(&claude_md)?;
            let already_points = content.contains(&format!("@{}", p.agents_md.display()));
            let has_content = !content.trim().is_empty();
            has_content && !already_points
        } else {
            false
        };

        if should_migrate {
            // Case 2: migrate existing CLAUDE.md
            let from_dir = p.behaviors_dir.join("from-claude");
            std::fs::create_dir_all(&from_dir)?;
            let rules = from_dir.join("RULES.md");
            std::fs::copy(&claude_md, &rules)?;

            let mut md = AgentsMd::parse(AGENTS_MD_HEADER);
            md.add("from-claude".into(), rules.clone());
            md.save(&p.agents_md)?;

            output::ok(&format!("Migrated CLAUDE.md → {}", rules.display()));
            output::ok(&format!("Created {}", p.agents_md.display()));
            output::note("");
            output::note(&format!(
                "Your old instructions live at:  {}",
                rules.display()
            ));
            output::note(&format!(
                "Do not edit:                    {}",
                claude_md.display()
            ));
            output::note(&format!(
                "Do not edit:                    {}",
                p.agents_md.display()
            ));
            output::note("Both are managed by lore.");
        } else {
            // Case 1: clean install (or recovery)
            let mut md = AgentsMd::parse(AGENTS_MD_HEADER);

            // Re-register any behaviors already on disk (recovery path)
            if p.behaviors_dir.exists() {
                let mut entries: Vec<_> = std::fs::read_dir(&p.behaviors_dir)?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .collect();
                entries.sort_by_key(|e| e.file_name());
                for entry in entries {
                    let bname = entry.file_name().to_string_lossy().to_string();
                    if let Ok(ep) = behavior_entry(&entry.path()) {
                        md.add(bname.clone(), ep);
                        output::warn(&format!("Re-registered existing behavior: {bname}"));
                    }
                }
            }

            md.save(&p.agents_md)?;
            output::ok(&format!("Created {}", p.agents_md.display()));
        }
    } else {
        output::ok("AGENTS.md exists — skipping");
    }

    // ── Migrate existing real skills dir ──────────────────────────────────────

    if claude_skills.exists() && !symlink::is_link(&claude_skills) {
        let mut moved = 0usize;
        let mut collision = false;

        for entry in std::fs::read_dir(&claude_skills)? {
            let entry = entry?;
            let name = entry.file_name();
            let dst = p.skills_dir.join(&name);
            if dst.exists() || symlink::is_link(&dst) {
                output::warn(&format!(
                    "Skill '{}' already exists in {} — skipping",
                    name.to_string_lossy(),
                    p.skills_dir.display()
                ));
                collision = true;
            } else {
                std::fs::rename(entry.path(), &dst)?;
                moved += 1;
            }
        }

        if moved > 0 {
            output::ok(&format!(
                "Moved {moved} skill(s) to {}",
                p.skills_dir.display()
            ));
        }

        // Try to remove the now-empty dir
        let _ = std::fs::remove_dir(&claude_skills);

        if collision && claude_skills.exists() && !symlink::is_link(&claude_skills) {
            anyhow::bail!(
                "{} still has unresolved skill collisions (see warnings above).\nResolve conflicts manually, then re-run: lore init",
                claude_skills.display()
            );
        }
    }

    // ── Wire Claude ───────────────────────────────────────────────────────────

    wire::wire_claude_dir(&p.agents_md, &p.skills_dir, &claude_dir)?;

    // ── Register account ─────────────────────────────────────────────────────

    if !config.accounts.contains_key(&account_name) {
        config.accounts.insert(
            account_name.clone(),
            claude_dir.to_string_lossy().into_owned(),
        );
        config.save(&config_path)?;
        output::ok(&format!("Registered account: {account_name}"));
    }

    Ok(())
}
