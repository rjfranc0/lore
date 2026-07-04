use crate::{
    agents_md::{AgentsMd, behavior_entry},
    config::{self, LoreConfig},
    output,
    paths::Paths,
    symlink, wire,
};
use anyhow::Result;

pub fn add(names: Vec<String>, account: Option<String>) -> Result<()> {
    match account {
        None => add_shared(names),
        Some(account) => add_scoped(names, &account),
    }
}

pub fn remove(names: Vec<String>, account: Option<String>) -> Result<()> {
    match account {
        None => remove_shared(names),
        Some(account) => remove_scoped(names, &account),
    }
}

fn add_shared(names: Vec<String>) -> Result<()> {
    let p = Paths::load()?;
    if !p.agents_md.exists() {
        anyhow::bail!("Run 'lore init' first");
    }
    let cwd = std::env::current_dir()?;
    let mut md = AgentsMd::load(&p.agents_md)?;

    for raw in names {
        let name = raw.trim_end_matches('/').to_string();
        let src = cwd.join(&name);
        let dst = p.behaviors_dir.join(&name);

        if !src.is_dir() {
            anyhow::bail!("'{}' not found in {}", name, cwd.display());
        }

        if !symlink::is_link(&dst) {
            symlink::create(&src, &dst)?;
            output::ok(&format!("Linked behavior {name}"));
        }

        let entry = behavior_entry(&dst)?;

        if !md.contains_path(&entry) {
            md.add(name.clone(), entry);
            md.save(&p.agents_md)?;
            output::ok(&format!("Added {name} to AGENTS.md"));
        } else {
            output::warn(&format!("{name} already in AGENTS.md"));
        }
    }
    Ok(())
}

fn remove_shared(names: Vec<String>) -> Result<()> {
    let p = Paths::load()?;

    for raw in names {
        let name = raw.trim_end_matches('/').to_string();
        let dst = p.behaviors_dir.join(&name);

        if symlink::is_link(&dst) {
            let mut md = AgentsMd::load(&p.agents_md)?;
            md.remove_by_name(&name);
            md.save(&p.agents_md)?;
            std::fs::remove_file(&dst)?;
            output::ok(&format!("Removed behavior {name}"));
        } else if dst.is_dir() {
            output::warn(&format!("{name} is a built-in behavior — remove manually:"));
            output::note(&format!("rm -rf {}", dst.display()));
            output::note(&format!(
                "Then remove its <!-- {name} --> block from {}",
                p.agents_md.display()
            ));
        } else {
            output::warn(&format!("{name} is not installed"));
        }
    }
    Ok(())
}

fn add_scoped(names: Vec<String>, account: &str) -> Result<()> {
    config::validate_account_name(account)?;
    let config = LoreConfig::load_or_default(&LoreConfig::config_path())?;
    let claude_dir = config.require_account_path(account)?;

    let lore_md = wire::lore_md_path(&claude_dir);
    if !lore_md.exists() {
        anyhow::bail!(
            "account '{account}' has no LORE.md — run `lore init --account {account}` first"
        );
    }

    let behaviors_dir = wire::claude_behaviors_path(&claude_dir);
    std::fs::create_dir_all(&behaviors_dir)?;

    let cwd = std::env::current_dir()?;
    let mut md = AgentsMd::load(&lore_md)?;

    for raw in names {
        let name = raw.trim_end_matches('/').to_string();
        let src = cwd.join(&name);
        let dst = behaviors_dir.join(&name);

        if !src.is_dir() {
            anyhow::bail!("'{}' not found in {}", name, cwd.display());
        }

        if !symlink::is_link(&dst) {
            symlink::create(&src, &dst)?;
            output::ok(&format!("Linked behavior {name}"));
        }

        let entry = behavior_entry(&dst)?;

        if !md.contains_path(&entry) {
            md.add(name.clone(), entry);
            md.save(&lore_md)?;
            output::ok(&format!("Added {name} to LORE.md"));
        } else {
            output::warn(&format!("{name} already in LORE.md"));
        }
    }
    Ok(())
}

fn remove_scoped(names: Vec<String>, account: &str) -> Result<()> {
    config::validate_account_name(account)?;
    let config = LoreConfig::load_or_default(&LoreConfig::config_path())?;
    let claude_dir = config.require_account_path(account)?;

    let behaviors_dir = wire::claude_behaviors_path(&claude_dir);
    let lore_md = wire::lore_md_path(&claude_dir);

    for raw in names {
        let name = raw.trim_end_matches('/').to_string();
        let dst = behaviors_dir.join(&name);

        if symlink::is_link(&dst) {
            let mut md = AgentsMd::load(&lore_md)?;
            md.remove_by_name(&name);
            md.save(&lore_md)?;
            std::fs::remove_file(&dst)?;
            output::ok(&format!("Removed behavior {name}"));
        } else if dst.is_dir() {
            output::warn(&format!("{name} is a built-in behavior — remove manually:"));
            output::note(&format!("rm -rf {}", dst.display()));
            output::note(&format!(
                "Then remove its <!-- {name} --> block from {}",
                lore_md.display()
            ));
        } else {
            output::warn(&format!("{name} is not installed in account '{account}'"));
        }
    }
    Ok(())
}
