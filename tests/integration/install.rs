use std::fs;
use crate::helpers::{Env, make_skill};

#[test]
fn creates_symlink_to_skill() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let src = env.home.path().join("src");
    make_skill(&src, "my-skill");

    env.lore().arg("install").arg("my-skill")
        .current_dir(&src).assert().success();

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

    env.lore().arg("install").arg("dup").current_dir(&src).assert().success();
    env.lore().arg("install").arg("dup").current_dir(&src)
        .assert().success()
        .stdout(predicates::str::contains("already installed"));
}

#[test]
fn exits_1_for_missing_skill() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.lore().arg("install").arg("does-not-exist")
        .current_dir(env.home.path())
        .assert().failure();
}

#[test]
fn normalizes_trailing_slash() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let src = env.home.path().join("src");
    make_skill(&src, "tabskill");

    env.lore().arg("install").arg("tabskill/")
        .current_dir(&src).assert().success();

    assert!(env.agents_dir.join("skills/tabskill").is_symlink());
}

#[test]
fn shared_install_with_no_registered_accounts_creates_only_shared_symlink() {
    let env = Env::bare();

    let src = env.home.path().join("src");
    make_skill(&src, "solo-skill");

    env.lore().arg("install").arg("solo-skill")
        .current_dir(&src).assert().success();

    assert!(env.agents_dir.join("skills/solo-skill").is_symlink());
}

#[test]
fn shared_install_relinks_into_every_registered_account() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");

    let src = env.home.path().join("src");
    make_skill(&src, "shared-skill");

    env.lore().arg("install").arg("shared-skill")
        .current_dir(&src).assert().success();

    assert!(env.agents_dir.join("skills/shared-skill").is_symlink());
    let work_link = env.account_skills("work").join("shared-skill");
    assert!(work_link.is_symlink());
    assert_eq!(fs::read_link(&work_link).unwrap(), env.agents_dir.join("skills/shared-skill"));
    assert!(env.claude_skills().join("shared-skill").is_symlink());
}

#[test]
fn account_install_creates_symlink_in_account_skills_only() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");

    let src = env.home.path().join("src");
    make_skill(&src, "work-skill");

    env.lore().arg("install").arg("work-skill").arg("--account").arg("work")
        .current_dir(&src).assert().success();

    assert!(env.account_skills("work").join("work-skill").is_symlink());
    assert!(!env.agents_dir.join("skills/work-skill").exists());
    assert!(!env.claude_skills().join("work-skill").exists());
}

#[test]
fn account_default_install_scopes_to_claude_skills_only() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");

    let src = env.home.path().join("src");
    make_skill(&src, "default-scoped-skill");

    env.lore().arg("install").arg("default-scoped-skill").arg("--account").arg("default")
        .current_dir(&src).assert().success();

    assert!(env.claude_skills().join("default-scoped-skill").is_symlink());
    assert!(!env.agents_dir.join("skills/default-scoped-skill").exists());
    assert!(!env.account_skills("work").join("default-scoped-skill").exists());
}

#[test]
fn account_install_on_unregistered_account_fails_and_creates_nothing() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let src = env.home.path().join("src");
    make_skill(&src, "ghost-skill");

    env.lore().arg("install").arg("ghost-skill").arg("--account").arg("ghost")
        .current_dir(&src).assert().failure();

    assert!(!env.home.path().join(".claude-ghost").exists());
    assert!(!env.agents_dir.join("skills/ghost-skill").exists());
}

#[test]
fn account_install_warns_on_duplicate_does_not_overwrite() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");

    let src = env.home.path().join("src");
    make_skill(&src, "dup-work");

    env.lore().arg("install").arg("dup-work").arg("--account").arg("work")
        .current_dir(&src).assert().success();
    env.lore().arg("install").arg("dup-work").arg("--account").arg("work")
        .current_dir(&src)
        .assert().success()
        .stdout(predicates::str::contains("already installed"));
}

#[test]
fn account_install_normalizes_trailing_slash() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");

    let src = env.home.path().join("src");
    make_skill(&src, "tabskill-work");

    env.lore().arg("install").arg("tabskill-work/").arg("--account").arg("work")
        .current_dir(&src).assert().success();

    assert!(env.account_skills("work").join("tabskill-work").is_symlink());
}
