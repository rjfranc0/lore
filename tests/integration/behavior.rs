use crate::helpers::{Env, make_behavior};
use std::fs;

#[test]
fn add_creates_symlink_and_agents_md_block() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let bsrc = env.home.path().join("bsrc");
    make_behavior(&bsrc, "my-rules", "RULES.md");

    env.lore()
        .arg("behavior")
        .arg("add")
        .arg("my-rules")
        .current_dir(&bsrc)
        .assert()
        .success();

    assert!(env.agents_dir.join("behaviors/my-rules").is_symlink());

    let agents_md = fs::read_to_string(env.agents_md()).unwrap();
    assert!(agents_md.contains("<!-- my-rules -->"));
    assert!(agents_md.contains(&format!(
        "@{}",
        env.agents_dir.join("behaviors/my-rules/RULES.md").display()
    )));
}

#[test]
fn rules_md_takes_priority_over_readme() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let bsrc = env.home.path().join("bsrc");
    let dir = bsrc.join("mixed");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("README.md"), "readme\n").unwrap();
    fs::write(dir.join("RULES.md"), "rules\n").unwrap();

    env.lore()
        .arg("behavior")
        .arg("add")
        .arg("mixed")
        .current_dir(&bsrc)
        .assert()
        .success();

    let agents_md = fs::read_to_string(env.agents_md()).unwrap();
    assert!(agents_md.contains("RULES.md"));
    assert!(!agents_md.contains("README.md"));
}

#[test]
fn add_is_idempotent() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let bsrc = env.home.path().join("bsrc");
    make_behavior(&bsrc, "once", "RULES.md");

    env.lore()
        .arg("behavior")
        .arg("add")
        .arg("once")
        .current_dir(&bsrc)
        .assert()
        .success();
    env.lore()
        .arg("behavior")
        .arg("add")
        .arg("once")
        .current_dir(&bsrc)
        .assert()
        .success();

    let agents_md = fs::read_to_string(env.agents_md()).unwrap();
    let count = agents_md.matches("<!-- once -->").count();
    assert_eq!(count, 1);
}

#[test]
fn add_normalizes_trailing_slash() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let bsrc = env.home.path().join("bsrc");
    make_behavior(&bsrc, "tabbed", "RULES.md");

    env.lore()
        .arg("behavior")
        .arg("add")
        .arg("tabbed/")
        .current_dir(&bsrc)
        .assert()
        .success();

    assert!(env.agents_dir.join("behaviors/tabbed").is_symlink());
}

#[test]
fn add_fails_with_helpful_message_before_init() {
    let env = Env::new();
    let bsrc = env.home.path().join("bsrc");
    make_behavior(&bsrc, "early", "RULES.md");

    env.lore()
        .arg("behavior")
        .arg("add")
        .arg("early")
        .current_dir(&bsrc)
        .assert()
        .failure()
        .stderr(predicates::str::contains("lore init"));
}

#[test]
fn remove_removes_symlink_and_agents_md_block() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let bsrc = env.home.path().join("bsrc");
    make_behavior(&bsrc, "bye", "RULES.md");

    env.lore()
        .arg("behavior")
        .arg("add")
        .arg("bye")
        .current_dir(&bsrc)
        .assert()
        .success();
    env.lore()
        .arg("behavior")
        .arg("remove")
        .arg("bye")
        .assert()
        .success();

    assert!(!env.agents_dir.join("behaviors/bye").is_symlink());
    let agents_md = fs::read_to_string(env.agents_md()).unwrap();
    assert!(!agents_md.contains("<!-- bye -->"));
}

#[test]
fn remove_exact_match_no_clobber_regex_special_name() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let bsrc = env.home.path().join("bsrc");
    make_behavior(&bsrc, "a.c", "RULES.md");
    make_behavior(&bsrc, "axc", "RULES.md");

    env.lore()
        .arg("behavior")
        .arg("add")
        .arg("a.c")
        .arg("axc")
        .current_dir(&bsrc)
        .assert()
        .success();
    env.lore()
        .arg("behavior")
        .arg("remove")
        .arg("a.c")
        .assert()
        .success();

    let agents_md = fs::read_to_string(env.agents_md()).unwrap();
    assert!(!agents_md.contains("<!-- a.c -->"));
    assert!(agents_md.contains("<!-- axc -->"));
    assert!(env.agents_dir.join("behaviors/axc").is_symlink());
}

#[test]
fn remove_normalizes_trailing_slash() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let bsrc = env.home.path().join("bsrc");
    make_behavior(&bsrc, "trailme", "RULES.md");

    env.lore()
        .arg("behavior")
        .arg("add")
        .arg("trailme")
        .current_dir(&bsrc)
        .assert()
        .success();
    env.lore()
        .arg("behavior")
        .arg("remove")
        .arg("trailme/")
        .assert()
        .success();

    let agents_md = fs::read_to_string(env.agents_md()).unwrap();
    assert!(!agents_md.contains("<!-- trailme -->"));
}
