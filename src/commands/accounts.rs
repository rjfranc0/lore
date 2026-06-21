use anyhow::Result;

use crate::{config::LoreConfig, output, paths::Paths, symlink, wire};

pub fn list() -> Result<()> {
    let config = LoreConfig::load_or_default(&LoreConfig::config_path())?;

    println!("Accounts:");
    if config.accounts.is_empty() {
        println!("  (none)");
    } else {
        for (name, path) in &config.accounts {
            println!("  {name:<12} → {path}");
        }
    }
    Ok(())
}

pub fn remove(name: String) -> Result<()> {
    let config_path = LoreConfig::config_path();
    let mut config = LoreConfig::load_or_default(&config_path)?;

    if !config.accounts.contains_key(&name) {
        output::warn(&format!("{name} is not a registered account"));
        return Ok(());
    }

    if name == "default" {
        output::warn("Removing 'default' — re-run 'lore init' to register it again");
    }

    config.accounts.remove(&name);
    config.save(&config_path)?;
    output::ok(&format!("Removed account: {name}"));
    Ok(())
}

pub fn sync() -> Result<()> {
    let config_path = LoreConfig::config_path();
    let config = LoreConfig::load_or_default(&config_path)?;
    let p = Paths::from_config(&config);

    let mut rewired = 0usize;
    for (name, path) in &config.accounts {
        let claude_dir = std::path::PathBuf::from(path);
        let claude_md = wire::claude_md_path(&claude_dir);
        let claude_skills = wire::claude_skills_path(&claude_dir);
        let lore_md = wire::lore_md_path(&claude_dir);

        // A read failure here (permission denied, non-UTF8) is folded into "not
        // wired" deliberately — sync is self-healing, so any unreadable state
        // routes through the same rewire path below rather than needing its own branch.
        let already_wired = claude_md.exists()
            && std::fs::read_to_string(&claude_md)
                .is_ok_and(|c| c.lines().any(|l| l.trim() == format!("@{}", lore_md.display())))
            && lore_md.exists()
            && std::fs::read_to_string(&lore_md)
                .is_ok_and(|c| c.lines().any(|l| l.trim() == format!("@{}", p.agents_md.display())))
            && claude_skills.is_dir()
            && !symlink::is_link(&claude_skills);

        if !already_wired {
            let (migration_behaviors_dir, migration_register_md) = if name == "default" {
                (p.behaviors_dir.clone(), p.agents_md.clone())
            } else {
                (claude_dir.join("behaviors"), wire::lore_md_path(&claude_dir))
            };
            wire::wire_claude_dir(
                &p.agents_md,
                &p.skills_dir,
                &claude_dir,
                &migration_behaviors_dir,
                &migration_register_md,
            )?;
            output::ok(&format!("Re-wired account: {name} → {}", claude_dir.display()));
            rewired += 1;
        }
    }

    if rewired == 0 {
        output::ok("Accounts already in sync");
    }
    Ok(())
}
