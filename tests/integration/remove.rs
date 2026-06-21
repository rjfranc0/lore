use crate::helpers::{Env, make_skill};

#[test]
fn removes_symlink_source_dir_untouched() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let src = env.home.path().join("src");
    make_skill(&src, "gone");

    env.lore().arg("install").arg("gone").current_dir(&src).assert().success();
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

    env.lore().arg("install").arg("tabremove").current_dir(&src).assert().success();
    env.lore().arg("remove").arg("tabremove/").assert().success();

    assert!(!env.agents_dir.join("skills/tabremove").is_symlink());
}

#[test]
fn warns_but_exits_0_when_not_installed() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.lore().arg("remove").arg("nonexistent")
        .assert().success()
        .stdout(predicates::str::contains("not installed"));
}

#[test]
fn shared_remove_deletes_symlink_and_account_relinks() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");

    let src = env.home.path().join("src");
    make_skill(&src, "shared-gone");
    env.lore().arg("install").arg("shared-gone").current_dir(&src).assert().success();
    assert!(env.account_skills("work").join("shared-gone").is_symlink());

    env.lore().arg("remove").arg("shared-gone").assert().success();

    assert!(!env.agents_dir.join("skills/shared-gone").exists());
    assert!(!env.account_skills("work").join("shared-gone").exists());
}

#[test]
fn account_remove_only_affects_targeted_account() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");
    env.register_account("personal");

    let src = env.home.path().join("src");
    make_skill(&src, "multi-acct-skill");
    env.lore().arg("install").arg("multi-acct-skill").current_dir(&src).assert().success();

    env.lore().arg("remove").arg("multi-acct-skill").arg("--account").arg("work").assert().success();

    assert!(!env.account_skills("work").join("multi-acct-skill").exists());
    assert!(env.agents_dir.join("skills/multi-acct-skill").is_symlink());
    assert!(env.account_skills("personal").join("multi-acct-skill").is_symlink());
}

#[test]
fn account_remove_normalizes_trailing_slash() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");

    let src = env.home.path().join("src");
    make_skill(&src, "tabremove-work");
    env.lore().arg("install").arg("tabremove-work").arg("--account").arg("work")
        .current_dir(&src).assert().success();

    env.lore().arg("remove").arg("tabremove-work/").arg("--account").arg("work").assert().success();

    assert!(!env.account_skills("work").join("tabremove-work").is_symlink());
}

#[test]
fn account_remove_warns_but_exits_0_when_not_installed() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");

    env.lore().arg("remove").arg("nonexistent").arg("--account").arg("work")
        .assert().success()
        .stdout(predicates::str::contains("not installed"));
}
