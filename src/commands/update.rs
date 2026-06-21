use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::{
    agents_md::{AgentsMd, behavior_entry},
    output,
    paths::Paths,
    symlink,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Skill,
    Behavior,
}

pub fn run(name: Option<String>, all: bool, path: Option<String>) -> Result<()> {
    match (name, all) {
        (Some(_), true) => anyhow::bail!("specify either <name> or --all, not both"),
        (None, false) => anyhow::bail!("specify a skill/behavior name or use --all"),
        (Some(name), false) => {
            let p = Paths::load()?;
            let cwd = std::env::current_dir()?;
            update_one(&p, &cwd, &name, path)
        }
        (None, true) => {
            let p = Paths::load()?;
            update_all(&p)
        }
    }
}

fn update_one(p: &Paths, cwd: &Path, name: &str, path: Option<String>) -> Result<()> {
    let (dst, kind) = locate(p, name)
        .ok_or_else(|| anyhow::anyhow!("'{name}' is not installed as a skill or behavior"))?;

    let src = match path {
        Some(path) => PathBuf::from(path),
        None => cwd.join(name),
    };

    if !src.is_dir() {
        anyhow::bail!("'{}' not found", src.display());
    }

    relink(&src, &dst)?;
    output::ok(&format!("Relinked {name} → {}", src.display()));

    if kind == Kind::Behavior {
        let result = AgentsMd::load(&p.agents_md)
            .and_then(|mut md| sync_behavior_entry(&mut md, &p.agents_md, name, &dst));
        warn_on_sync_failure(name, result);
    }

    Ok(())
}

fn update_all(p: &Paths) -> Result<()> {
    let mut candidates: Vec<(String, PathBuf, PathBuf, Kind)> = Vec::new();
    candidates.extend(
        find_broken(&p.skills_dir)?
            .into_iter()
            .map(|(n, d, t)| (n, d, t, Kind::Skill)),
    );
    candidates.extend(
        find_broken(&p.behaviors_dir)?
            .into_iter()
            .map(|(n, d, t)| (n, d, t, Kind::Behavior)),
    );

    if candidates.is_empty() {
        output::ok("No broken symlinks found");
        return Ok(());
    }

    let needs_agents_md = candidates
        .iter()
        .any(|(_, _, _, kind)| *kind == Kind::Behavior);
    let mut md = if needs_agents_md {
        match AgentsMd::load(&p.agents_md) {
            Ok(md) => Some(md),
            Err(e) => {
                output::warn(&format!(
                    "Could not load AGENTS.md, behavior entries will not be updated: {e}"
                ));
                None
            }
        }
    } else {
        None
    };

    for (name, dst, old_target, kind) in candidates {
        println!("{name} → {} (broken)", old_target.display());
        print!("  new path (blank to skip): ");
        io::stdout().flush()?;

        let mut buf = String::new();
        io::stdin().lock().read_line(&mut buf)?;

        relink_candidate(&name, &dst, &buf, kind, md.as_mut(), &p.agents_md)?;
    }

    Ok(())
}

fn warn_on_sync_failure(name: &str, result: Result<()>) {
    if let Err(e) = result {
        output::warn(&format!("Could not update AGENTS.md entry for {name}: {e}"));
    }
}

/// Applies one `--all` prompt response: blank skips, a non-directory warns and skips,
/// otherwise relinks. Factored out from `update_all`'s loop so the decision logic is
/// testable without driving real stdin.
fn relink_candidate(
    name: &str,
    dst: &Path,
    input: &str,
    kind: Kind,
    md: Option<&mut AgentsMd>,
    agents_md_path: &Path,
) -> Result<()> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        output::warn(&format!("Skipped {name}"));
        return Ok(());
    }

    let src = PathBuf::from(trimmed);
    if !src.is_dir() {
        output::warn(&format!(
            "'{}' is not a directory, skipped {name}",
            src.display()
        ));
        return Ok(());
    }

    relink(&src, dst)?;
    output::ok(&format!("Relinked {name} → {}", src.display()));

    if let (Kind::Behavior, Some(md)) = (kind, md) {
        warn_on_sync_failure(name, sync_behavior_entry(md, agents_md_path, name, dst));
    }

    Ok(())
}

fn locate(p: &Paths, name: &str) -> Option<(PathBuf, Kind)> {
    let skill_path = p.skills_dir.join(name);
    if symlink::is_link(&skill_path) || skill_path.is_dir() {
        return Some((skill_path, Kind::Skill));
    }

    let behavior_path = p.behaviors_dir.join(name);
    if symlink::is_link(&behavior_path) || behavior_path.is_dir() {
        return Some((behavior_path, Kind::Behavior));
    }

    None
}

fn relink(src: &Path, dst: &Path) -> Result<()> {
    if symlink::is_link(dst) {
        std::fs::remove_file(dst)?;
    }
    symlink::create(src, dst)
}

fn sync_behavior_entry(
    md: &mut AgentsMd,
    agents_md_path: &Path,
    name: &str,
    dst: &Path,
) -> Result<()> {
    let new_entry = behavior_entry(dst)?;
    let unchanged = md
        .behaviors
        .iter()
        .any(|b| b.name == name && b.path == new_entry);

    if md.contains_name(name) && !unchanged {
        md.remove_by_name(name);
        md.add(name.to_string(), new_entry.clone());
        md.save(agents_md_path)?;
        output::ok(&format!(
            "Updated AGENTS.md entry for {name} → {}",
            new_entry.display()
        ));
    }

    Ok(())
}

fn find_broken(dir: &Path) -> Result<Vec<(String, PathBuf, PathBuf)>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut broken = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if symlink::is_link(&path) && !symlink::is_live(&path) {
            let target = std::fs::read_link(&path)?;
            broken.push((
                entry.file_name().to_string_lossy().into_owned(),
                path,
                target,
            ));
        }
    }
    Ok(broken)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_paths(tmp: &tempfile::TempDir) -> Paths {
        Paths {
            agents_dir: tmp.path().to_path_buf(),
            skills_dir: tmp.path().join("skills"),
            behaviors_dir: tmp.path().join("behaviors"),
            agents_md: tmp.path().join("AGENTS.md"),
        }
    }

    #[test]
    fn locate_prefers_skills_dir_over_behaviors_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let p = test_paths(&tmp);
        std::fs::create_dir_all(&p.skills_dir).unwrap();
        std::fs::create_dir_all(&p.behaviors_dir).unwrap();

        let src = tmp.path().join("src-dir");
        std::fs::create_dir_all(&src).unwrap();
        symlink::create(&src, &p.skills_dir.join("dup")).unwrap();
        symlink::create(&src, &p.behaviors_dir.join("dup")).unwrap();

        let (path, kind) = locate(&p, "dup").unwrap();
        assert_eq!(kind, Kind::Skill);
        assert_eq!(path, p.skills_dir.join("dup"));
    }

    #[test]
    fn update_one_force_relinks_healthy_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let p = test_paths(&tmp);
        std::fs::create_dir_all(&p.skills_dir).unwrap();

        let old_src = tmp.path().join("old-src");
        std::fs::create_dir_all(&old_src).unwrap();
        symlink::create(&old_src, &p.skills_dir.join("my-skill")).unwrap();

        let cwd = tmp.path().join("cwd");
        let new_src = cwd.join("my-skill");
        std::fs::create_dir_all(&new_src).unwrap();

        update_one(&p, &cwd, "my-skill", None).unwrap();

        let resolved = std::fs::read_link(p.skills_dir.join("my-skill")).unwrap();
        assert_eq!(resolved, new_src);
    }

    #[test]
    fn update_one_uses_explicit_path_over_cwd() {
        let tmp = tempfile::tempdir().unwrap();
        let p = test_paths(&tmp);
        std::fs::create_dir_all(&p.skills_dir).unwrap();

        let old_src = tmp.path().join("old-src");
        std::fs::create_dir_all(&old_src).unwrap();
        symlink::create(&old_src, &p.skills_dir.join("my-skill")).unwrap();

        // cwd/my-skill deliberately does not exist — only --path's target does.
        let cwd = tmp.path().join("cwd");
        std::fs::create_dir_all(&cwd).unwrap();
        let explicit_src = tmp.path().join("explicit-src");
        std::fs::create_dir_all(&explicit_src).unwrap();

        update_one(
            &p,
            &cwd,
            "my-skill",
            Some(explicit_src.to_string_lossy().into_owned()),
        )
        .unwrap();

        let resolved = std::fs::read_link(p.skills_dir.join("my-skill")).unwrap();
        assert_eq!(resolved, explicit_src);
    }

    #[test]
    fn run_errors_when_neither_name_nor_all_given() {
        assert!(run(None, false, None).is_err());
    }

    #[test]
    fn run_errors_when_both_name_and_all_given() {
        assert!(run(Some("x".into()), true, None).is_err());
    }

    #[test]
    fn update_one_errors_when_not_installed() {
        let tmp = tempfile::tempdir().unwrap();
        let p = test_paths(&tmp);
        std::fs::create_dir_all(&p.skills_dir).unwrap();
        std::fs::create_dir_all(&p.behaviors_dir).unwrap();
        let cwd = tmp.path().join("cwd");

        assert!(update_one(&p, &cwd, "ghost", None).is_err());
    }

    #[test]
    fn sync_behavior_entry_updates_when_resolved_path_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let agents_md_path = tmp.path().join("AGENTS.md");

        let behavior_dir = tmp.path().join("my-behavior");
        std::fs::create_dir_all(&behavior_dir).unwrap();
        std::fs::write(behavior_dir.join("README.md"), "hi").unwrap();

        let old_entry = tmp.path().join("old-path/RULES.md");
        let mut md = AgentsMd::parse("<!-- managed by lore -->\n");
        md.add("my-behavior".to_string(), old_entry.clone());

        sync_behavior_entry(&mut md, &agents_md_path, "my-behavior", &behavior_dir).unwrap();

        let new_entry = behavior_dir.join("README.md");
        assert!(md.contains_path(&new_entry));
        assert!(!md.contains_path(&old_entry));
        assert!(agents_md_path.exists());
    }

    #[test]
    fn sync_behavior_entry_leaves_unchanged_entry_untouched() {
        let tmp = tempfile::tempdir().unwrap();
        let agents_md_path = tmp.path().join("AGENTS.md");

        let behavior_dir = tmp.path().join("my-behavior");
        std::fs::create_dir_all(&behavior_dir).unwrap();
        std::fs::write(behavior_dir.join("RULES.md"), "hi").unwrap();

        let entry = behavior_dir.join("RULES.md");
        let mut md = AgentsMd::parse("<!-- managed by lore -->\n");
        md.add("my-behavior".to_string(), entry);

        sync_behavior_entry(&mut md, &agents_md_path, "my-behavior", &behavior_dir).unwrap();

        // No-op: nothing changed, so no save should have happened.
        assert!(!agents_md_path.exists());
    }

    #[test]
    fn update_one_warns_but_succeeds_when_behavior_entry_unresolvable() {
        let tmp = tempfile::tempdir().unwrap();
        let p = test_paths(&tmp);
        std::fs::create_dir_all(&p.behaviors_dir).unwrap();

        let old_src = tmp.path().join("old-src");
        std::fs::create_dir_all(&old_src).unwrap();
        std::fs::write(old_src.join("RULES.md"), "hi").unwrap();
        symlink::create(&old_src, &p.behaviors_dir.join("my-behavior")).unwrap();

        let mut md = AgentsMd::parse("<!-- managed by lore -->\n");
        md.add("my-behavior".to_string(), old_src.join("RULES.md"));
        md.save(&p.agents_md).unwrap();

        // New location has no resolvable .md entry at all.
        let cwd = tmp.path().join("cwd");
        let new_src = cwd.join("my-behavior");
        std::fs::create_dir_all(&new_src).unwrap();

        let result = update_one(&p, &cwd, "my-behavior", None);
        assert!(
            result.is_ok(),
            "a bookkeeping failure must not undo a successful relink"
        );

        let resolved = std::fs::read_link(p.behaviors_dir.join("my-behavior")).unwrap();
        assert_eq!(resolved, new_src);
    }

    #[test]
    fn relink_candidate_skips_on_blank_input() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("dangling-link");
        symlink::create(&tmp.path().join("does-not-exist"), &dst).unwrap();

        let mut md = AgentsMd::parse("");
        let agents_md_path = tmp.path().join("AGENTS.md");

        relink_candidate(
            "my-skill",
            &dst,
            "\n",
            Kind::Skill,
            Some(&mut md),
            &agents_md_path,
        )
        .unwrap();

        assert!(symlink::is_link(&dst));
        assert!(!symlink::is_live(&dst));
    }

    #[test]
    fn relink_candidate_skips_and_warns_on_non_directory_input() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("dangling-link");
        symlink::create(&tmp.path().join("does-not-exist"), &dst).unwrap();

        let not_a_dir = tmp.path().join("just-a-file");
        std::fs::write(&not_a_dir, "x").unwrap();

        let mut md = AgentsMd::parse("");
        let agents_md_path = tmp.path().join("AGENTS.md");

        relink_candidate(
            "my-skill",
            &dst,
            &not_a_dir.to_string_lossy(),
            Kind::Skill,
            Some(&mut md),
            &agents_md_path,
        )
        .unwrap();

        assert!(symlink::is_link(&dst));
        assert!(!symlink::is_live(&dst));
    }

    #[test]
    fn relink_candidate_warns_but_succeeds_when_behavior_entry_unresolvable() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("dangling-link");
        symlink::create(&tmp.path().join("does-not-exist"), &dst).unwrap();

        let new_src = tmp.path().join("new-src-no-md");
        std::fs::create_dir_all(&new_src).unwrap();

        let mut md = AgentsMd::parse("<!-- managed by lore -->\n");
        let agents_md_path = tmp.path().join("AGENTS.md");

        let result = relink_candidate(
            "my-behavior",
            &dst,
            &new_src.to_string_lossy(),
            Kind::Behavior,
            Some(&mut md),
            &agents_md_path,
        );

        assert!(result.is_ok());
        assert_eq!(std::fs::read_link(&dst).unwrap(), new_src);
    }

    #[test]
    fn relink_candidate_continues_for_behavior_when_agents_md_unavailable() {
        let tmp = tempfile::tempdir().unwrap();
        let dst = tmp.path().join("dangling-link");
        symlink::create(&tmp.path().join("does-not-exist"), &dst).unwrap();

        let new_src = tmp.path().join("new-src");
        std::fs::create_dir_all(&new_src).unwrap();
        std::fs::write(new_src.join("RULES.md"), "hi").unwrap();

        let agents_md_path = tmp.path().join("AGENTS.md");

        let result = relink_candidate(
            "my-behavior",
            &dst,
            &new_src.to_string_lossy(),
            Kind::Behavior,
            None,
            &agents_md_path,
        );

        assert!(result.is_ok());
        assert_eq!(std::fs::read_link(&dst).unwrap(), new_src);
    }
}
