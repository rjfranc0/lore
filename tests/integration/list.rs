use std::fs;
use predicates::prelude::PredicateBooleanExt;
use crate::helpers::{Env, make_skill};

#[test]
fn shows_installed_skill_symlink() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let src = env.home.path().join("src");
    make_skill(&src, "visible");

    env.lore().arg("install").arg("visible").current_dir(&src).assert().success();
    env.lore().arg("list")
        .assert().success()
        .stdout(predicates::str::contains("visible"))
        .stdout(predicates::str::contains("✗ broken").not());
}

#[test]
fn flags_broken_skill_symlink() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let gone_src = env.home.path().join("gone-src");
    make_skill(&gone_src, "ghost");

    env.lore().arg("install").arg("ghost").current_dir(&gone_src).assert().success();
    fs::remove_dir_all(&gone_src).unwrap();

    env.lore().arg("list")
        .assert().success()
        .stdout(predicates::str::contains("ghost"))
        .stdout(predicates::str::contains("✗ broken"));
}

#[test]
fn shows_real_dir_skill_as_migrated() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    // Simulate a real dir in skills (as created by init migration)
    let migrated = env.agents_dir.join("skills/migrated-skill");
    fs::create_dir_all(&migrated).unwrap();
    fs::write(migrated.join("SKILL.md"), "").unwrap();

    env.lore().arg("list")
        .assert().success()
        .stdout(predicates::str::contains("migrated-skill"))
        .stdout(predicates::str::contains("(migrated)"));
}

#[test]
fn shows_built_in_behavior_as_built_in() {
    let env = Env::new();
    fs::write(env.claude_md(), "# old\n").unwrap();
    env.lore().arg("init").assert().success();

    env.lore().arg("list")
        .assert().success()
        .stdout(predicates::str::contains("from-claude"))
        .stdout(predicates::str::contains("(built-in)"));
}
