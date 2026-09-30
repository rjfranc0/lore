use crate::{config::LoreConfig, paths::Paths, symlink, wire};
use anyhow::Result;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
enum EntryKind {
    Dir,
    MdFile,
}

pub fn run() -> Result<()> {
    let config = LoreConfig::load_or_default(&LoreConfig::config_path())?;
    let p = Paths::from_config(&config);

    println!("Shared skills:");
    print_dir_entries(&p.skills_dir, "  ", "(migrated)", None, EntryKind::Dir)?;

    println!();
    println!("Shared behaviors:");
    print_dir_entries(&p.behaviors_dir, "  ", "(built-in)", None, EntryKind::Dir)?;

    println!();
    println!("Shared agents:");
    print_dir_entries(&p.subagents_dir, "  ", "", None, EntryKind::MdFile)?;

    for (name, claude_dir) in &config.accounts {
        let claude_dir = PathBuf::from(claude_dir);

        let (skills_out, has_skills) = collect_dir_entries(
            &wire::claude_skills_path(&claude_dir),
            "    ",
            "(migrated)",
            Some(&p.skills_dir),
            EntryKind::Dir,
        )?;
        let (behaviors_out, has_behaviors) = collect_dir_entries(
            &wire::claude_behaviors_path(&claude_dir),
            "    ",
            "(built-in)",
            None,
            EntryKind::Dir,
        )?;
        let (agents_out, has_agents) = collect_dir_entries(
            &wire::claude_agents_path(&claude_dir),
            "    ",
            "",
            Some(&p.subagents_dir),
            EntryKind::MdFile,
        )?;

        // `default` is registered on every `init`, so it's the one account
        // that must stay silent when it has nothing account-specific to show
        // — otherwise every install would grow an empty "Account: default"
        // section. Any other registered account always gets its section
        // (see `shows_none_for_empty_account_sections`).
        if name == "default" && !has_skills && !has_behaviors && !has_agents {
            continue;
        }

        println!();
        println!("Account: {name}");

        println!("  Skills:");
        print!("{skills_out}");

        println!("  Behaviors:");
        print!("{behaviors_out}");

        println!("  Agents:");
        print!("{agents_out}");
    }

    Ok(())
}

/// Prints every entry in `dir` via [`collect_dir_entries`].
fn print_dir_entries(
    dir: &Path,
    indent: &str,
    real_dir_label: &str,
    skip_relink_target: Option<&Path>,
    kind: EntryKind,
) -> Result<()> {
    let (out, _) = collect_dir_entries(dir, indent, real_dir_label, skip_relink_target, kind)?;
    print!("{out}");
    Ok(())
}

/// Renders every entry in `dir`, one per line: symlinks show their target
/// (flagged `✗ broken` when dead), real directories show `real_dir_label`.
/// With `EntryKind::MdFile` (agents), liveness means "resolves to a file", the
/// displayed name drops the `.md` extension, and non-symlink entries are never
/// listed — hand-written agent files are the user's, not lore's.
/// When `skip_relink_target` is `Some(shared_dir)`, entries whose symlink
/// target is exactly `shared_dir/<name>` are omitted — these are shared-skill
/// re-links (see `wire::relink_skill`), not account-owned entries. Renders
/// `(none)` if nothing qualified. Returns the rendered text alongside
/// whether any qualifying entry was found, so callers can decide whether a
/// section is worth showing at all.
fn collect_dir_entries(
    dir: &Path,
    indent: &str,
    real_dir_label: &str,
    skip_relink_target: Option<&Path>,
    kind: EntryKind,
) -> Result<(String, bool)> {
    let mut out = String::new();
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
                let live = match kind {
                    EntryKind::Dir => symlink::is_live(&path),
                    EntryKind::MdFile => symlink::is_live_file(&path),
                };
                let suffix = if live { "" } else { "  ✗ broken" };
                let name = name.to_string_lossy();
                let display_name = match kind {
                    EntryKind::Dir => &*name,
                    EntryKind::MdFile => name.strip_suffix(".md").unwrap_or(&name),
                };
                let _ = writeln!(
                    out,
                    "{indent}{display_name:<24} → {}{suffix}",
                    target.display()
                );
                found = true;
            } else if matches!(kind, EntryKind::Dir) && path.is_dir() {
                let _ = writeln!(
                    out,
                    "{indent}{:<24}   {real_dir_label}",
                    name.to_string_lossy()
                );
                found = true;
            }
        }
    }
    if !found {
        let _ = writeln!(out, "{indent}(none)");
    }
    Ok((out, found))
}
