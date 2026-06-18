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
        let claude_md = claude_dir.join("CLAUDE.md");
        let claude_skills = claude_dir.join("skills");

        let already_wired = claude_md.exists()
            && std::fs::read_to_string(&claude_md)
                .is_ok_and(|content| content.contains(&format!("@{}", p.agents_md.display())))
            && symlink::is_link(&claude_skills)
            && symlink::is_live(&claude_skills);

        if !already_wired {
            wire::wire_claude_dir(&p.agents_md, &p.skills_dir, &claude_dir)?;
            output::ok(&format!("Re-wired account: {name} → {}", claude_dir.display()));
            rewired += 1;
        }
    }

    if rewired == 0 {
        output::ok("Accounts already in sync");
    }
    Ok(())
}
