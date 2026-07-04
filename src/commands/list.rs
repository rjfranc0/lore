use crate::{config::LoreConfig, paths::Paths, symlink, wire};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn run() -> Result<()> {
    let config = LoreConfig::load_or_default(&LoreConfig::config_path())?;
    let p = Paths::from_config(&config);

    println!("Shared skills:");
    print_dir_entries(&p.skills_dir, "  ", "(migrated)", None)?;

    println!();
    println!("Shared behaviors:");
    print_dir_entries(&p.behaviors_dir, "  ", "(built-in)", None)?;

    for (name, claude_dir) in config
        .accounts
        .iter()
        .filter(|(name, _)| name.as_str() != "default")
    {
        let claude_dir = PathBuf::from(claude_dir);

        println!();
        println!("Account: {name}");

        println!("  Skills:");
        print_dir_entries(
            &wire::claude_skills_path(&claude_dir),
            "    ",
            "(migrated)",
            Some(&p.skills_dir),
        )?;

        println!("  Behaviors:");
        print_dir_entries(
            &wire::claude_behaviors_path(&claude_dir),
            "    ",
            "(built-in)",
            None,
        )?;
    }

    Ok(())
}

/// Prints every entry in `dir`, one per line: symlinks show their target
/// (flagged `✗ broken` when dead), real directories show `real_dir_label`.
/// When `skip_relink_target` is `Some(shared_dir)`, entries whose symlink
/// target is exactly `shared_dir/<name>` are omitted — these are shared-skill
/// re-links (see `wire::relink_skill`), not account-owned entries. Prints
/// `(none)` if nothing qualified.
fn print_dir_entries(
    dir: &Path,
    indent: &str,
    real_dir_label: &str,
    skip_relink_target: Option<&Path>,
) -> Result<()> {
    let mut found = false;
    if dir.exists() {
        let mut entries: Vec<_> = std::fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name();
            if symlink::is_link(&path) {
                let target = std::fs::read_link(&path)?;
                if skip_relink_target.is_some_and(|shared_dir| target == shared_dir.join(&name)) {
                    continue;
                }
                let suffix = if symlink::is_live(&path) {
                    String::new()
                } else {
                    "  ✗ broken".to_string()
                };
                println!(
                    "{indent}{:<24} → {}{suffix}",
                    name.to_string_lossy(),
                    target.display()
                );
                found = true;
            } else if path.is_dir() {
                println!("{indent}{:<24}   {real_dir_label}", name.to_string_lossy());
                found = true;
            }
        }
    }
    if !found {
        println!("{indent}(none)");
    }
    Ok(())
}
