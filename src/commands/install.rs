use crate::{output, paths::Paths, symlink};
use anyhow::Result;

pub fn run(skills: Vec<String>) -> Result<()> {
    let p = Paths::load()?;
    let cwd = std::env::current_dir()?;

    for raw in skills {
        let name = raw.trim_end_matches('/');
        let src = cwd.join(name);
        let dst = p.skills_dir.join(name);

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
    }
    Ok(())
}
