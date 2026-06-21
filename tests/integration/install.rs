use crate::helpers::{Env, make_skill};
use std::fs;

#[test]
fn creates_symlink_to_skill() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let src = env.home.path().join("src");
    make_skill(&src, "my-skill");

    env.lore()
        .arg("install")
        .arg("my-skill")
        .current_dir(&src)
        .assert()
        .success();

    let link = env.agents_dir.join("skills/my-skill");
    assert!(link.is_symlink());
    assert_eq!(fs::read_link(&link).unwrap(), src.join("my-skill"));
}

#[test]
fn warns_on_duplicate_does_not_overwrite() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let src = env.home.path().join("src");
    make_skill(&src, "dup");

    env.lore()
        .arg("install")
        .arg("dup")
        .current_dir(&src)
        .assert()
        .success();
    env.lore()
        .arg("install")
        .arg("dup")
        .current_dir(&src)
        .assert()
        .success()
        .stdout(predicates::str::contains("already installed"));
}

#[test]
fn exits_1_for_missing_skill() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.lore()
        .arg("install")
        .arg("does-not-exist")
        .current_dir(env.home.path())
        .assert()
        .failure();
}

#[test]
fn normalizes_trailing_slash() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let src = env.home.path().join("src");
    make_skill(&src, "tabskill");

    env.lore()
        .arg("install")
        .arg("tabskill/")
        .current_dir(&src)
        .assert()
        .success();

    assert!(env.agents_dir.join("skills/tabskill").is_symlink());
}
