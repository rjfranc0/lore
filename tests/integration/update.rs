use crate::helpers::{Env, make_behavior, make_skill};
use predicates::prelude::PredicateBooleanExt;
use std::fs;
use std::path::PathBuf;

// macOS reports `current_dir()` canonicalized (`/var` resolves to
// `/private/var`), but `TempDir::path()` returns the raw, non-canonical
// form. Any path we expect a `cwd`-derived symlink target to match has to
// be built from this canonical root, or the comparison is flaky on macOS.
fn home(env: &Env) -> PathBuf {
    fs::canonicalize(env.home.path()).unwrap()
}

#[test]
fn relinks_skill_from_new_location_after_source_move() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let old_src = home(&env).join("old-src");
    make_skill(&old_src, "moved-skill");
    env.lore()
        .arg("install")
        .arg("moved-skill")
        .current_dir(&old_src)
        .assert()
        .success();

    let new_src = home(&env).join("new-src");
    fs::rename(&old_src, &new_src).unwrap();

    env.lore()
        .arg("update")
        .arg("moved-skill")
        .current_dir(&new_src)
        .assert()
        .success();

    let link = env.agents_dir.join("skills/moved-skill");
    assert_eq!(fs::read_link(&link).unwrap(), new_src.join("moved-skill"));
    assert!(
        new_src.join("moved-skill/SKILL.md").exists(),
        "source file must survive the relink untouched"
    );

    env.lore()
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("✗ broken").not());
}

#[test]
fn relinks_with_explicit_path_flag_instead_of_cwd() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let old_src = home(&env).join("old-src");
    make_skill(&old_src, "flagged-skill");
    env.lore()
        .arg("install")
        .arg("flagged-skill")
        .current_dir(&old_src)
        .assert()
        .success();

    // The real new source — note it is NOT inside `unrelated_dir`, the cwd
    // the command runs from, so a pass here can only mean `--path` was used.
    let new_src = home(&env).join("elsewhere");
    make_skill(&new_src, "flagged-skill");
    fs::remove_dir_all(&old_src).unwrap();

    let unrelated_dir = home(&env).join("unrelated");
    fs::create_dir_all(&unrelated_dir).unwrap();

    env.lore()
        .arg("update")
        .arg("flagged-skill")
        .arg("--path")
        .arg(new_src.join("flagged-skill"))
        .current_dir(&unrelated_dir)
        .assert()
        .success();

    let link = env.agents_dir.join("skills/flagged-skill");
    assert_eq!(fs::read_link(&link).unwrap(), new_src.join("flagged-skill"));
}

#[test]
fn force_relinks_already_healthy_skill_symlink() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let src_a = home(&env).join("src-a");
    make_skill(&src_a, "healthy-skill");
    env.lore()
        .arg("install")
        .arg("healthy-skill")
        .current_dir(&src_a)
        .assert()
        .success();

    let link = env.agents_dir.join("skills/healthy-skill");
    assert!(
        link.is_symlink() && fs::read_link(&link).unwrap().is_dir(),
        "precondition: symlink starts healthy"
    );

    let src_b = home(&env).join("src-b");
    make_skill(&src_b, "healthy-skill");

    env.lore()
        .arg("update")
        .arg("healthy-skill")
        .current_dir(&src_b)
        .assert()
        .success();

    assert_eq!(fs::read_link(&link).unwrap(), src_b.join("healthy-skill"));
}

#[test]
fn updates_agents_md_path_when_behavior_entry_filename_changes() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let bsrc_a = home(&env).join("bsrc-a");
    make_behavior(&bsrc_a, "renamed-entry", "RULES.md");
    env.lore()
        .arg("behavior")
        .arg("add")
        .arg("renamed-entry")
        .current_dir(&bsrc_a)
        .assert()
        .success();

    let bsrc_b = home(&env).join("bsrc-b");
    make_behavior(&bsrc_b, "renamed-entry", "README.md");

    env.lore()
        .arg("update")
        .arg("renamed-entry")
        .current_dir(&bsrc_b)
        .assert()
        .success();

    let agents_md = fs::read_to_string(env.agents_md()).unwrap();
    assert!(agents_md.contains(&format!(
        "@{}",
        env.agents_dir.join("behaviors/renamed-entry/README.md").display()
    )));
    assert!(!agents_md.contains(&format!(
        "@{}",
        env.agents_dir.join("behaviors/renamed-entry/RULES.md").display()
    )));
    assert_eq!(
        fs::read_to_string(bsrc_b.join("renamed-entry/README.md")).unwrap(),
        "rules\n",
        "source file must survive the relink untouched"
    );
}

#[test]
fn all_relinks_valid_answer_and_skips_blank_answer() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let skill_src_a = home(&env).join("skill-src-a");
    make_skill(&skill_src_a, "break-me-skill");
    env.lore()
        .arg("install")
        .arg("break-me-skill")
        .current_dir(&skill_src_a)
        .assert()
        .success();

    let behavior_src_a = home(&env).join("behavior-src-a");
    make_behavior(&behavior_src_a, "break-me-behavior", "RULES.md");
    env.lore()
        .arg("behavior")
        .arg("add")
        .arg("break-me-behavior")
        .current_dir(&behavior_src_a)
        .assert()
        .success();

    let skill_src_b = home(&env).join("skill-src-b");
    make_skill(&skill_src_b, "break-me-skill");
    fs::remove_dir_all(&skill_src_a).unwrap();
    fs::remove_dir_all(&behavior_src_a).unwrap();

    // `update_all` always scans skills_dir's broken candidates before
    // behaviors_dir's (see `commands/update.rs::update_all`), so with exactly
    // one broken entry per dir, the skill prompt is always first.
    env.lore()
        .arg("update")
        .arg("--all")
        .write_stdin(format!(
            "{}\n\n",
            skill_src_b.join("break-me-skill").display()
        ))
        .assert()
        .success()
        .stdout(predicates::str::contains("Skipped break-me-behavior"));

    let skill_link = env.agents_dir.join("skills/break-me-skill");
    assert_eq!(
        fs::read_link(&skill_link).unwrap(),
        skill_src_b.join("break-me-skill")
    );

    let behavior_link = env.agents_dir.join("behaviors/break-me-behavior");
    assert_eq!(
        fs::read_link(&behavior_link).unwrap(),
        behavior_src_a.join("break-me-behavior")
    );
}

#[test]
fn all_reports_nothing_broken() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    env.lore()
        .arg("update")
        .arg("--all")
        .assert()
        .success()
        .stdout(predicates::str::contains("No broken symlinks found"));
}

#[test]
fn no_name_and_no_all_fails_with_clear_error() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    env.lore()
        .arg("update")
        .assert()
        .failure()
        .stderr(predicates::str::contains("--all"));
}
