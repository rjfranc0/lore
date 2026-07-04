use std::path::Path;

use anyhow::Result;

use crate::{config::LoreConfig, output, paths::Paths, symlink, wire};

pub fn run(skills: Vec<String>, account: Option<String>) -> Result<()> {
    let config = LoreConfig::load_or_default(&LoreConfig::config_path())?;

    match &account {
        None => remove_shared(&skills, &config),
        Some(name) => remove_scoped(&skills, &config, name),
    }
}

fn remove_shared(skills: &[String], config: &LoreConfig) -> Result<()> {
    let p = Paths::from_config(config);
    for raw in skills {
        let name = raw.trim_end_matches('/');
        let dst = p.skills_dir.join(name);

        if symlink::is_link(&dst) {
            std::fs::remove_file(&dst)?;
            output::ok(&format!("Removed {name}"));
        } else {
            output::warn(&format!("{name} is not installed"));
        }

        for account_dir in config.accounts.values() {
            wire::unlink_account_skill(Path::new(account_dir), name)?;
        }
    }
    Ok(())
}

fn remove_scoped(skills: &[String], config: &LoreConfig, account: &str) -> Result<()> {
    let claude_dir = config.require_account_path(account)?;
    let claude_skills = wire::claude_skills_path(&claude_dir);

    for raw in skills {
        let name = raw.trim_end_matches('/');
        let dst = claude_skills.join(name);

        if symlink::is_link(&dst) {
            std::fs::remove_file(&dst)?;
            output::ok(&format!("Removed {name}"));
        } else {
            output::warn(&format!("{name} is not installed in account '{account}'"));
        }
    }
    Ok(())
}
