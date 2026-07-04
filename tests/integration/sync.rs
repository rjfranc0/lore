use crate::helpers::{Env, make_behavior};
use std::fs;

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
    assert!(agents_md.contains(&format!(
        "@{}",
        env.agents_dir.join("behaviors/new-b/RULES.md").display()
    )));
}

#[test]
fn removes_stale_entry_when_behavior_dir_gone() {
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

    env.lore()
        .arg("behavior")
        .arg("add")
        .arg("steady")
        .current_dir(&bsrc)
        .assert()
        .success();
    env.lore()
        .arg("sync")
        .assert()
        .success()
        .stdout(predicates::str::contains("Already in sync"));

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
    agents_md.push_str(&format!(
        "\n<!-- from-claude -->\n@{}/behaviors/from-claude/RULES.md\n",
        env.agents_dir.display()
    ));
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
    env.lore()
        .arg("sync")
        .assert()
        .failure()
        .stderr(predicates::str::contains("lore init"));
}

#[test]
fn adds_behavior_present_in_account_dir_not_yet_in_lore_md() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");

    let new_b = env.account_behaviors("work").join("new-rule");
    fs::create_dir_all(&new_b).unwrap();
    fs::write(new_b.join("RULES.md"), "rules\n").unwrap();

    env.lore()
        .arg("sync")
        .assert()
        .success()
        .stdout(predicates::str::contains("Added work/new-rule to LORE.md"));

    let lore_md = fs::read_to_string(env.account_lore_md("work")).unwrap();
    assert!(lore_md.contains("<!-- new-rule -->"));
    assert!(lore_md.contains(&format!("@{}", new_b.join("RULES.md").display())));
}

#[test]
fn removes_stale_entry_from_account_lore_md_when_symlink_gone() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");

    let bsrc = env.home.path().join("bsrc");
    make_behavior(&bsrc, "bye", "RULES.md");
    env.lore()
        .arg("behavior")
        .arg("add")
        .arg("bye")
        .arg("--account")
        .arg("work")
        .current_dir(&bsrc)
        .assert()
        .success();

    fs::remove_file(env.account_behaviors("work").join("bye")).unwrap();

    env.lore()
        .arg("sync")
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "Removed stale entry from work: bye",
        ));

    let lore_md = fs::read_to_string(env.account_lore_md("work")).unwrap();
    assert!(!lore_md.contains("<!-- bye -->"));
}

#[test]
fn re_adds_agents_md_header_when_stripped_from_account_lore_md() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");

    let lore_md_path = env.account_lore_md("work");
    let stripped = fs::read_to_string(&lore_md_path)
        .unwrap()
        .lines()
        .skip(1)
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&lore_md_path, stripped).unwrap();

    env.lore()
        .arg("sync")
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "Re-added AGENTS.md header to work's LORE.md",
        ));

    let lore_md = fs::read_to_string(&lore_md_path).unwrap();
    assert!(lore_md.lines().next().unwrap().starts_with('@'));
    assert!(
        lore_md
            .lines()
            .next()
            .unwrap()
            .contains(&env.agents_md().display().to_string())
    );
}

#[test]
fn warns_and_skips_account_with_missing_lore_md() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");

    fs::remove_file(env.account_lore_md("work")).unwrap();

    let new_b = env.agents_dir.join("behaviors/shared-rule");
    fs::create_dir_all(&new_b).unwrap();
    fs::write(new_b.join("RULES.md"), "rules\n").unwrap();

    env.lore()
        .arg("sync")
        .assert()
        .success()
        .stdout(predicates::str::contains("has no LORE.md"))
        .stdout(predicates::str::contains("Added shared-rule to AGENTS.md"));

    let agents_md = fs::read_to_string(env.agents_md()).unwrap();
    assert!(agents_md.contains("<!-- shared-rule -->"));
}

#[test]
fn reconciles_multiple_accounts_touching_only_the_drifted_one() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");
    env.register_account("personal");

    let new_b = env.account_behaviors("work").join("new-rule");
    fs::create_dir_all(&new_b).unwrap();
    fs::write(new_b.join("RULES.md"), "rules\n").unwrap();

    let output = env.lore().arg("sync").assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Added work/new-rule to LORE.md"));
    assert!(stdout.contains("account: personal — already in sync"));
    assert!(!stdout.contains("account: work — already in sync"));
}

#[test]
fn reports_already_in_sync_once_when_nothing_drifted() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");
    env.register_account("personal");

    env.lore()
        .arg("sync")
        .assert()
        .success()
        .stdout(predicates::str::contains("Already in sync"));
}

#[test]
fn emits_granular_in_sync_lines_when_something_else_changed() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");
    env.register_account("personal");

    let new_b = env.agents_dir.join("behaviors/shared-rule");
    fs::create_dir_all(&new_b).unwrap();
    fs::write(new_b.join("RULES.md"), "rules\n").unwrap();

    let account_new_b = env.account_behaviors("work").join("work-rule");
    fs::create_dir_all(&account_new_b).unwrap();
    fs::write(account_new_b.join("RULES.md"), "rules\n").unwrap();

    let output = env.lore().arg("sync").assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("Added shared-rule to AGENTS.md"));
    assert!(stdout.contains("Added work/work-rule to LORE.md"));
    assert!(stdout.contains("account: personal — already in sync"));
    assert!(!stdout.contains("AGENTS.md already in sync"));
    assert!(!stdout.contains("account: work — already in sync"));
}
