use crate::{
    agents_md::{AgentsMd, behavior_entry},
    config::LoreConfig,
    output,
    paths::Paths,
    wire,
};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn run() -> Result<()> {
    let config = LoreConfig::load_or_default(&LoreConfig::config_path())?;
    let p = Paths::from_config(&config);
    if !p.agents_md.exists() {
        anyhow::bail!("Run 'lore init' first");
    }

    let mut total_changes = 0usize;

    // Pass 1: AGENTS.md
    let mut agents_md = AgentsMd::load(&p.agents_md)?;
    let (added, removed) = reconcile_behaviors(&mut agents_md, &p.behaviors_dir)?;
    for name in &removed {
        output::ok(&format!("Removed stale entry: {name}"));
    }
    for name in &added {
        output::ok(&format!("Added {name} to AGENTS.md"));
    }
    let agents_changed = !added.is_empty() || !removed.is_empty();
    if agents_changed {
        agents_md.save(&p.agents_md)?;
        total_changes += added.len() + removed.len();
    }

    // Pass 2: per-account LORE.md
    let mut accounts_in_sync: Vec<String> = Vec::new();
    for (name, path) in &config.accounts {
        let claude_dir = PathBuf::from(path);
        let lore_md = wire::lore_md_path(&claude_dir);
        if !lore_md.exists() {
            output::warn(&format!(
                "account '{name}' has no LORE.md — skipping (run `lore init --account {name}`)"
            ));
            continue;
        }

        let mut md = AgentsMd::load(&lore_md)?;

        let want_header = format!("@{}\n", p.agents_md.display());
        let header_changed = md.header.lines().next() != want_header.lines().next();
        if header_changed {
            md.header = want_header;
            output::ok(&format!("Re-added AGENTS.md header to {name}'s LORE.md"));
        }

        let behaviors_dir = wire::claude_behaviors_path(&claude_dir);
        let (added, removed) = reconcile_behaviors(&mut md, &behaviors_dir)?;
        for entry_name in &removed {
            output::ok(&format!("Removed stale entry from {name}: {entry_name}"));
        }
        for entry_name in &added {
            output::ok(&format!("Added {name}/{entry_name} to LORE.md"));
        }

        let changed = header_changed || !added.is_empty() || !removed.is_empty();
        if changed {
            md.save(&lore_md)?;
            total_changes += added.len() + removed.len() + if header_changed { 1 } else { 0 };
        } else {
            accounts_in_sync.push(name.clone());
        }
    }

    if total_changes == 0 {
        output::ok("Already in sync");
        return Ok(());
    }

    if !agents_changed {
        output::ok("AGENTS.md already in sync");
    }
    for name in &accounts_in_sync {
        output::ok(&format!("account: {name} — already in sync"));
    }

    Ok(())
}

/// Reconciles `md` in place against what's actually on disk under
/// `behaviors_dir`: drops entries whose backing dir is gone, adds entries for
/// dirs present but not yet registered. Doesn't print or save — shared by the
/// AGENTS.md pass and the per-account LORE.md pass, which each own their own
/// messaging and persistence.
fn reconcile_behaviors(
    md: &mut AgentsMd,
    behaviors_dir: &Path,
) -> Result<(Vec<String>, Vec<String>)> {
    let removed: Vec<String> = md
        .behaviors
        .iter()
        .filter(|b| !behaviors_dir.join(&b.name).is_dir())
        .map(|b| b.name.clone())
        .collect();
    for name in &removed {
        md.remove_by_name(name);
    }

    let mut added = Vec::new();
    if behaviors_dir.exists() {
        let mut entries: Vec<_> = std::fs::read_dir(behaviors_dir)?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if let Ok(ep) = behavior_entry(&path)
                && !md.contains_path(&ep)
            {
                let name = entry.file_name().to_string_lossy().to_string();
                md.add(name.clone(), ep);
                added.push(name);
            }
        }
    }

    Ok((added, removed))
}
