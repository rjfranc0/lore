use crate::{
    agents_md::{AgentsMd, behavior_entry},
    config::{self, LoreConfig},
    output,
    paths::Paths,
    symlink, wire,
};
use anyhow::Result;
use std::path::Path;

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
    link_and_register(names, &p.behaviors_dir, &p.agents_md, "AGENTS.md")
}

fn remove_shared(names: Vec<String>) -> Result<()> {
    let p = Paths::load()?;
    unlink_and_deregister(names, &p.behaviors_dir, &p.agents_md, |name| {
        format!("{name} is not installed")
    })
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

    link_and_register(names, &behaviors_dir, &lore_md, "LORE.md")
}

fn remove_scoped(names: Vec<String>, account: &str) -> Result<()> {
    config::validate_account_name(account)?;
    let config = LoreConfig::load_or_default(&LoreConfig::config_path())?;
    let claude_dir = config.require_account_path(account)?;

    let behaviors_dir = wire::claude_behaviors_path(&claude_dir);
    let lore_md = wire::lore_md_path(&claude_dir);

    unlink_and_deregister(names, &behaviors_dir, &lore_md, |name| {
        format!("{name} is not installed in account '{account}'")
    })
}

/// Symlinks each behavior into `behaviors_dir` and registers it in `md_path`
/// (idempotent), shared by both the `AGENTS.md` and per-account `LORE.md` paths.
fn link_and_register(
    names: Vec<String>,
    behaviors_dir: &Path,
    md_path: &Path,
    md_label: &str,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let mut md = AgentsMd::load(md_path)?;

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
            md.save(md_path)?;
            output::ok(&format!("Added {name} to {md_label}"));
        } else {
            output::warn(&format!("{name} already in {md_label}"));
        }
    }
    Ok(())
}

/// Removes each behavior's symlink from `behaviors_dir` and deregisters it from
/// `md_path`, shared by both the `AGENTS.md` and per-account `LORE.md` paths.
fn unlink_and_deregister(
    names: Vec<String>,
    behaviors_dir: &Path,
    md_path: &Path,
    not_installed: impl Fn(&str) -> String,
) -> Result<()> {
    for raw in names {
        let name = raw.trim_end_matches('/').to_string();
        let dst = behaviors_dir.join(&name);

        if symlink::is_link(&dst) {
            let mut md = AgentsMd::load(md_path)?;
            md.remove_by_name(&name);
            md.save(md_path)?;
            std::fs::remove_file(&dst)?;
            output::ok(&format!("Removed behavior {name}"));
        } else if dst.is_dir() {
            output::warn(&format!("{name} is a built-in behavior — remove manually:"));
            output::note(&format!("rm -rf {}", dst.display()));
            output::note(&format!(
                "Then remove its <!-- {name} --> block from {}",
                md_path.display()
            ));
        } else {
            output::warn(&not_installed(&name));
        }
    }
    Ok(())
}
