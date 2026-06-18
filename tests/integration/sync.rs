use std::fs;
use crate::helpers::{Env, make_behavior};

#[test]
fn adds_behavior_on_disk_not_yet_in_agents_md() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let new_b = env.agents_dir.join("behaviors/new-b");
    fs::create_dir_all(&new_b).unwrap();
    fs::write(new_b.join("RULES.md"), "rules\n").unwrap();

    env.lore().arg("sync").assert().success();

    let agents_md = fs::read_to_string(env.agents_md()).unwrap();
    assert!(agents_md.contains("<!-- new-b -->"));
    assert!(agents_md.contains(&format!("@{}", env.agents_dir.join("behaviors/new-b/RULES.md").display())));
}

#[test]
fn removes_stale_entry_when_behavior_dir_gone() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let bsrc = env.home.path().join("bsrc");
    make_behavior(&bsrc, "bye", "RULES.md");

    env.lore().arg("behavior").arg("add").arg("bye").current_dir(&bsrc).assert().success();

    // Remove the symlink (simulating the dir being gone)
    fs::remove_file(env.agents_dir.join("behaviors/bye")).unwrap();

    env.lore().arg("sync").assert().success();

    let agents_md = fs::read_to_string(env.agents_md()).unwrap();
    assert!(!agents_md.contains("<!-- bye -->"));
}

#[test]
fn no_op_when_already_in_sync() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    let bsrc = env.home.path().join("bsrc");
    make_behavior(&bsrc, "steady", "RULES.md");

    env.lore().arg("behavior").arg("add").arg("steady").current_dir(&bsrc).assert().success();
    env.lore().arg("sync")
        .assert().success()
        .stdout(predicates::str::contains("already in sync"));

    let agents_md = fs::read_to_string(env.agents_md()).unwrap();
    assert_eq!(agents_md.matches("<!-- steady -->").count(), 1);
}

#[test]
fn handles_split_scenario_removes_old_adds_new() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    // Simulate from-claude built-in
    let from_claude = env.agents_dir.join("behaviors/from-claude");
    fs::create_dir_all(&from_claude).unwrap();
    fs::write(from_claude.join("RULES.md"), "old rules\n").unwrap();
    let mut agents_md = fs::read_to_string(env.agents_md()).unwrap();
    agents_md.push_str(&format!("\n<!-- from-claude -->\n@{}/behaviors/from-claude/RULES.md\n", env.agents_dir.display()));
    fs::write(env.agents_md(), &agents_md).unwrap();

    // Create two new split behaviors
    let part_a = env.agents_dir.join("behaviors/part-a");
    fs::create_dir_all(&part_a).unwrap();
    fs::write(part_a.join("RULES.md"), "part a\n").unwrap();
    let part_b = env.agents_dir.join("behaviors/part-b");
    fs::create_dir_all(&part_b).unwrap();
    fs::write(part_b.join("RULES.md"), "part b\n").unwrap();

    // Delete old one
    fs::remove_dir_all(&from_claude).unwrap();

    env.lore().arg("sync").assert().success();

    let agents_md = fs::read_to_string(env.agents_md()).unwrap();
    assert!(!agents_md.contains("<!-- from-claude -->"));
    assert!(agents_md.contains("<!-- part-a -->"));
    assert!(agents_md.contains("<!-- part-b -->"));
}

#[test]
fn fails_with_helpful_message_before_init() {
    let env = Env::new();
    env.lore().arg("sync")
        .assert().failure()
        .stderr(predicates::str::contains("lore init"));
}
