use std::path::Path;

use anyhow::Result;

use crate::{
    config::{self, LoreConfig},
    output,
    paths::Paths,
    symlink, wire,
};

pub fn run(skills: Vec<String>, account: Option<String>) -> Result<()> {
    let config = LoreConfig::load_or_default(&LoreConfig::config_path())?;
    let cwd = std::env::current_dir()?;

    let claude_dir = match &account {
        None => None,
        Some(name) => {
            config::validate_account_name(name)?;
            Some(config.require_account_path(name)?)
        }
    };
    let skills_dir = match &claude_dir {
        Some(claude_dir) => wire::claude_skills_path(claude_dir),
        None => Paths::from_config(&config).skills_dir,
    };
    std::fs::create_dir_all(&skills_dir)?;

    for raw in &skills {
        let name = raw.trim_end_matches('/');
        install_one(&cwd, &skills_dir, name)?;

        if claude_dir.is_none() {
            for account_dir in config.accounts.values() {
                wire::relink_skill(&skills_dir, Path::new(account_dir), name)?;
            }
        }
    }
    Ok(())
}

fn install_one(cwd: &Path, skills_dir: &Path, name: &str) -> Result<()> {
    let src = cwd.join(name);
    let dst = skills_dir.join(name);

    if !src.is_dir() {
        anyhow::bail!("'{}' not found in {}", name, cwd.display());
    }

    if symlink::is_link(&dst) {
        output::warn(&format!("{name} already installed"));
        let existing = std::fs::read_link(&dst)?;
        output::note(&format!("existing  → {}", existing.display()));
        output::note(&format!("attempted → {}", src.display()));
    } else {
        symlink::create(&src, &dst)?;
        output::ok(&format!("Installed {name}"));
    }
    Ok(())
}
