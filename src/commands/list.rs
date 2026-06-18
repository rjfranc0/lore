use anyhow::Result;
use crate::{paths::Paths, symlink};

pub fn run() -> Result<()> {
    let p = Paths::load()?;

    println!("Skills:");
    let mut found = false;
    if p.skills_dir.exists() {
        let mut entries: Vec<_> = std::fs::read_dir(&p.skills_dir)?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name();
            if symlink::is_link(&path) {
                let target = std::fs::read_link(&path)?;
                let suffix = if symlink::is_live(&path) { String::new() } else { "  ✗ broken".to_string() };
                println!("  {:<24} → {}{suffix}", name.to_string_lossy(), target.display());
                found = true;
            } else if path.is_dir() {
                println!("  {:<24}   (migrated)", name.to_string_lossy());
                found = true;
            }
        }
    }
    if !found { println!("  (none)"); }

    println!();
    println!("Behaviors:");
    found = false;
    if p.behaviors_dir.exists() {
        let mut entries: Vec<_> = std::fs::read_dir(&p.behaviors_dir)?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name();
            if symlink::is_link(&path) {
                let target = std::fs::read_link(&path)?;
                let suffix = if symlink::is_live(&path) { String::new() } else { "  ✗ broken".to_string() };
                println!("  {:<24} → {}{suffix}", name.to_string_lossy(), target.display());
                found = true;
            } else if path.is_dir() {
                println!("  {:<24}   (built-in)", name.to_string_lossy());
                found = true;
            }
        }
    }
    if !found { println!("  (none)"); }

    Ok(())
}
