use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::{
    config::{self, LoreConfig},
    output,
    paths::Paths,
    symlink, wire,
};

pub fn add(names: Vec<String>, account: Option<String>) -> Result<()> {
    let config = LoreConfig::load_or_default(&LoreConfig::config_path())?;
    let cwd = std::env::current_dir()?;

    let (agents_dir, scoped) = match &account {
        None => (Paths::from_config(&config).subagents_dir, false),
        Some(name) => (scoped_agents_dir(&config, name)?, true),
    };
    std::fs::create_dir_all(&agents_dir)?;

    for raw in &names {
        let name = normalize_agent_name(raw);
        let file_name = format!("{name}.md");
        let src = cwd.join(&file_name);
        let dst = agents_dir.join(&file_name);

        if !src.is_file() {
            output::warn(&format!("'{file_name}' not found in {}", cwd.display()));
            continue;
        }

        if symlink::is_link(&dst) {
            output::warn(&format!("{name} already installed"));
            let existing = std::fs::read_link(&dst)?;
            output::note(&format!("existing  → {}", existing.display()));
            output::note(&format!("attempted → {}", src.display()));
        } else if dst.exists() {
            output::warn(&format!(
                "{name} exists in {} and is not a symlink — skipping",
                agents_dir.display()
            ));
        } else {
            symlink::create(&src, &dst)?;
            output::ok(&format!("Installed agent {name}"));
        }

        if !scoped {
            for account_dir in config.accounts.values() {
                wire::relink_agent(&agents_dir, Path::new(account_dir), &file_name)?;
            }
        }
    }
    Ok(())
}

pub fn remove(names: Vec<String>, account: Option<String>) -> Result<()> {
    let config = LoreConfig::load_or_default(&LoreConfig::config_path())?;

    match &account {
        None => remove_shared(&names, &config),
        Some(name) => remove_scoped(&names, &config, name),
    }
}

fn remove_shared(names: &[String], config: &LoreConfig) -> Result<()> {
    let p = Paths::from_config(config);
    for raw in names {
        let name = normalize_agent_name(raw);
        let file_name = format!("{name}.md");
        let dst = p.subagents_dir.join(&file_name);

        if symlink::is_link(&dst) {
            std::fs::remove_file(&dst)?;
            output::ok(&format!("Removed agent {name}"));
        } else {
            output::warn(&format!("{name} is not installed"));
        }

        for account_dir in config.accounts.values() {
            wire::unlink_account_agent(Path::new(account_dir), &p.subagents_dir, &file_name)?;
        }
    }
    Ok(())
}

fn remove_scoped(names: &[String], config: &LoreConfig, account: &str) -> Result<()> {
    let claude_agents = scoped_agents_dir(config, account)?;

    for raw in names {
        let name = normalize_agent_name(raw);
        let dst = claude_agents.join(format!("{name}.md"));

        if symlink::is_link(&dst) {
            std::fs::remove_file(&dst)?;
            output::ok(&format!("Removed agent {name}"));
        } else {
            output::warn(&format!("{name} is not installed in account '{account}'"));
        }
    }
    Ok(())
}

/// Resolves `--account <name>` to its `agents/` directory. A symlinked
/// `agents/` is the user's own setup (it may alias the shared pool or another
/// account), so a scoped install or removal refuses it rather than write
/// through it.
fn scoped_agents_dir(config: &LoreConfig, account: &str) -> Result<PathBuf> {
    config::validate_account_name(account)?;
    let claude_dir = config.require_account_path(account)?;
    if wire::agents_dir_is_symlink(&claude_dir) {
        bail!(
            "{} is a symlink — lore does not modify through a user-managed agents directory; \
             replace it with a real directory to use --account",
            wire::claude_agents_path(&claude_dir).display()
        );
    }
    Ok(wire::claude_agents_path(&claude_dir))
}

/// A trailing `.md` is stripped so `reviewer.md` (e.g. from shell tab-completion
/// of the file) is equivalent to `reviewer`.
fn normalize_agent_name(raw: &str) -> &str {
    let name = crate::commands::normalize_name(raw);
    name.strip_suffix(".md").unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_agent_name_strips_one_trailing_md() {
        assert_eq!(normalize_agent_name("reviewer"), "reviewer");
        assert_eq!(normalize_agent_name("reviewer.md"), "reviewer");
        assert_eq!(normalize_agent_name("reviewer.md.md"), "reviewer.md");
    }
}
