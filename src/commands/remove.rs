use anyhow::Result;
use crate::{output, paths::Paths, symlink};

pub fn run(skills: Vec<String>) -> Result<()> {
    let p = Paths::resolve();

    for raw in skills {
        let name = raw.trim_end_matches('/');
        let dst = p.skills_dir.join(name);

        if symlink::is_link(&dst) {
            std::fs::remove_file(&dst)?;
            output::ok(&format!("Removed {name}"));
        } else {
            output::warn(&format!("{name} is not installed"));
        }
    }
    Ok(())
}
