use anyhow::Result;
use crate::{agents_md::{AgentsMd, behavior_entry}, output, paths::Paths};

pub fn run() -> Result<()> {
    let p = Paths::resolve();
    if !p.agents_md.exists() {
        anyhow::bail!("Run 'lore init' first");
    }
    let mut md = AgentsMd::load(&p.agents_md)?;
    let mut added = 0usize;
    let mut removed = 0usize;

    // Remove stale entries (in AGENTS.md but dir gone from disk)
    let stale: Vec<String> = md.behaviors.iter()
        .filter(|b| !p.behaviors_dir.join(&b.name).is_dir())
        .map(|b| b.name.clone())
        .collect();
    for name in stale {
        md.remove_by_name(&name);
        output::ok(&format!("Removed stale entry: {name}"));
        removed += 1;
    }

    // Add missing entries (dir on disk but not in AGENTS.md)
    if p.behaviors_dir.exists() {
        let mut entries: Vec<_> = std::fs::read_dir(&p.behaviors_dir)?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            if !path.is_dir() { continue; }
            if let Ok(ep) = behavior_entry(&path) {
                if !md.contains_path(&ep) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    output::ok(&format!("Added {name} to AGENTS.md"));
                    md.add(name, ep);
                    added += 1;
                }
            }
        }
    }

    if added > 0 || removed > 0 {
        md.save(&p.agents_md)?;
    } else {
        output::ok("AGENTS.md already in sync");
    }
    Ok(())
}
