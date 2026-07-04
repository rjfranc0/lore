use crate::helpers::{Env, make_skill};

#[test]
fn removes_symlink_source_dir_untouched() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let src = env.home.path().join("src");
    make_skill(&src, "gone");

    env.lore()
        .arg("install")
        .arg("gone")
        .current_dir(&src)
        .assert()
        .success();
    env.lore().arg("remove").arg("gone").assert().success();

    assert!(!env.agents_dir.join("skills/gone").exists());
    assert!(src.join("gone").is_dir());
}

#[test]
fn normalizes_trailing_slash() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let src = env.home.path().join("src");
    make_skill(&src, "tabremove");

    env.lore()
        .arg("install")
        .arg("tabremove")
        .current_dir(&src)
        .assert()
        .success();
    env.lore()
        .arg("remove")
        .arg("tabremove/")
        .assert()
        .success();

    assert!(!env.agents_dir.join("skills/tabremove").is_symlink());
}

#[test]
fn warns_but_exits_0_when_not_installed() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.lore()
        .arg("remove")
        .arg("nonexistent")
        .assert()
        .success()
        .stdout(predicates::str::contains("not installed"));
}
