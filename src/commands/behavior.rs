use crate::{
    agents_md::{AgentsMd, behavior_entry},
    output,
    paths::Paths,
    symlink,
};
use anyhow::Result;

pub fn add(names: Vec<String>) -> Result<()> {
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

pub fn remove(names: Vec<String>) -> Result<()> {
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
