use crate::helpers::{Env, make_behavior, make_skill};
use predicates::prelude::PredicateBooleanExt;
use std::fs;

#[test]
fn shows_shared_headers_and_no_account_sections_by_default() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    env.lore()
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("Shared skills:"))
        .stdout(predicates::str::contains("Shared behaviors:"))
        .stdout(predicates::str::contains("Account:").not());
}

#[test]
fn shows_installed_skill_symlink() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let src = env.home.path().join("src");
    make_skill(&src, "visible");

    env.lore()
        .arg("install")
        .arg("visible")
        .current_dir(&src)
        .assert()
        .success();
    env.lore()
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("visible"))
        .stdout(predicates::str::contains("✗ broken").not());
}

#[test]
fn flags_broken_skill_symlink() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let gone_src = env.home.path().join("gone-src");
    make_skill(&gone_src, "ghost");

    env.lore()
        .arg("install")
        .arg("ghost")
        .current_dir(&gone_src)
        .assert()
        .success();
    fs::remove_dir_all(&gone_src).unwrap();

    env.lore()
        .arg("list")
        .assert()
        .success()
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

    env.lore()
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("migrated-skill"))
        .stdout(predicates::str::contains("(migrated)"));
}

#[test]
fn shows_built_in_behavior_as_built_in() {
    let env = Env::new();
    fs::write(env.claude_md(), "# old\n").unwrap();
    env.lore().arg("init").assert().success();

    env.lore()
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("from-claude"))
        .stdout(predicates::str::contains("(built-in)"));
}

#[test]
fn renders_account_section_for_each_registered_account() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");

    env.lore()
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("Account: work"));
}

#[test]
fn shows_none_for_empty_account_sections() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("personal");

    let output = env.lore().arg("list").output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    let account_section = &stdout[stdout.find("Account: personal").unwrap()..];
    assert_eq!(account_section.matches("(none)").count(), 2);
}

#[test]
fn account_installed_skill_shown_under_account_section_only() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");

    let src = env.home.path().join("src");
    make_skill(&src, "work-only-skill");

    env.lore()
        .arg("install")
        .arg("work-only-skill")
        .arg("--account")
        .arg("work")
        .current_dir(&src)
        .assert()
        .success();

    let output = env.lore().arg("list").output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let account_pos = stdout.find("Account: work").unwrap();

    assert!(!stdout[..account_pos].contains("work-only-skill"));
    let account_section = &stdout[account_pos..];
    assert!(account_section.contains("work-only-skill"));
    assert!(
        account_section.contains(
            &src.canonicalize()
                .unwrap()
                .join("work-only-skill")
                .display()
                .to_string()
        )
    );
}

#[test]
fn shared_skill_relink_not_duplicated_under_account_section() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");

    let src = env.home.path().join("src");
    make_skill(&src, "shared-listed-skill");

    env.lore()
        .arg("install")
        .arg("shared-listed-skill")
        .current_dir(&src)
        .assert()
        .success();

    let output = env.lore().arg("list").output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let account_section = &stdout[stdout.find("Account: work").unwrap()..];
    assert!(!account_section.contains("shared-listed-skill"));
}

#[test]
fn flags_broken_account_skill_symlink() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");

    let gone_src = env.home.path().join("gone-work-src");
    make_skill(&gone_src, "work-ghost");

    env.lore()
        .arg("install")
        .arg("work-ghost")
        .arg("--account")
        .arg("work")
        .current_dir(&gone_src)
        .assert()
        .success();
    fs::remove_dir_all(&gone_src).unwrap();

    env.lore()
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("work-ghost"))
        .stdout(predicates::str::contains("✗ broken"));
}

#[test]
fn shows_account_behavior_under_account_section() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");

    let bsrc = env.home.path().join("bsrc");
    make_behavior(&bsrc, "work-rules", "RULES.md");

    env.lore()
        .arg("behavior")
        .arg("add")
        .arg("work-rules")
        .arg("--account")
        .arg("work")
        .current_dir(&bsrc)
        .assert()
        .success();

    let output = env.lore().arg("list").output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let account_section = &stdout[stdout.find("Account: work").unwrap()..];
    assert!(account_section.contains("work-rules"));
}

#[test]
fn shows_migrated_from_claude_account_behavior_as_built_in() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let account_claude_dir = env.home.path().join(".claude-work");
    fs::create_dir_all(&account_claude_dir).unwrap();
    fs::write(account_claude_dir.join("CLAUDE.md"), "# old work rules\n").unwrap();
    env.register_account("work");

    let output = env.lore().arg("list").output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let account_section = &stdout[stdout.find("Account: work").unwrap()..];
    assert!(account_section.contains("from-claude"));
    assert!(account_section.contains("(built-in)"));
}

#[test]
fn renders_multiple_account_sections_in_alphabetical_order() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("zebra");
    env.register_account("alpha");

    let output = env.lore().arg("list").output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let alpha_pos = stdout.find("Account: alpha").unwrap();
    let zebra_pos = stdout.find("Account: zebra").unwrap();
    assert!(alpha_pos < zebra_pos);
}
