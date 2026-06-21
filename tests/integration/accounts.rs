use crate::helpers::Env;
use predicates::prelude::PredicateBooleanExt;

#[test]
fn list_shows_registered_accounts() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    env.lore()
        .arg("accounts")
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("default"))
        .stdout(predicates::str::contains(env.claude_dir.display().to_string()));
}

#[test]
fn list_shows_none_when_accounts_empty() {
    let env = Env::bare();

    env.lore()
        .arg("accounts")
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("(none)"));
}

#[test]
fn init_account_twice_is_idempotent_single_entry() {
    let env = Env::new();
    env.lore().arg("init").arg("--account").arg("work").assert().success();
    env.lore().arg("init").arg("--account").arg("work").assert().success();

    let config = std::fs::read_to_string(&env.config_path).unwrap();
    let work_entry = format!("work = \"{}\"", env.home.path().join(".claude-work").display());
    assert_eq!(config.lines().filter(|l| *l == work_entry).count(), 1);
}

#[test]
fn two_named_accounts_register_independently() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.lore().arg("init").arg("--account").arg("work").assert().success();
    env.lore().arg("init").arg("--account").arg("personal").assert().success();

    env.lore()
        .arg("accounts")
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("default"))
        .stdout(predicates::str::contains("work"))
        .stdout(predicates::str::contains("personal"));

    assert!(env.home.path().join(".claude-work/CLAUDE.md").is_file());
    assert!(env.home.path().join(".claude-personal/CLAUDE.md").is_file());
}

#[test]
fn named_account_creates_own_lore_md_importing_shared_agents_md() {
    let env = Env::new();
    env.lore().arg("init").arg("--account").arg("work").assert().success();

    let work_lore_md = env.home.path().join(".claude-work/LORE.md");
    assert!(work_lore_md.is_file());
    let content = std::fs::read_to_string(&work_lore_md).unwrap();
    assert!(content.contains(&format!("@{}", env.agents_md().display())));
}

#[test]
fn account_migration_registers_in_own_lore_md_not_shared_agents_md() {
    let env = Env::new();
    let work_dir = env.home.path().join(".claude-work");
    std::fs::create_dir_all(&work_dir).unwrap();
    std::fs::write(work_dir.join("CLAUDE.md"), "# work notes\nbe nice\n").unwrap();

    env.lore().arg("init").arg("--account").arg("work").assert().success();

    let rules = work_dir.join("behaviors/from-claude/RULES.md");
    assert!(rules.is_file());
    assert!(std::fs::read_to_string(&rules).unwrap().contains("work notes"));

    let work_lore_md = std::fs::read_to_string(work_dir.join("LORE.md")).unwrap();
    assert!(work_lore_md.contains("from-claude"));

    // Shared AGENTS.md must stay oblivious to this account-scoped migration.
    let agents_md = std::fs::read_to_string(env.agents_md()).unwrap();
    assert!(!agents_md.contains("from-claude"));
}

#[test]
fn account_migration_is_idempotent_single_from_claude_block() {
    let env = Env::new();
    let work_dir = env.home.path().join(".claude-work");
    std::fs::create_dir_all(&work_dir).unwrap();
    std::fs::write(work_dir.join("CLAUDE.md"), "# work notes\nbe nice\n").unwrap();

    env.lore().arg("init").arg("--account").arg("work").assert().success();
    let claude_md_after_first = std::fs::read_to_string(work_dir.join("CLAUDE.md")).unwrap();

    env.lore().arg("init").arg("--account").arg("work").assert().success();
    let claude_md_after_second = std::fs::read_to_string(work_dir.join("CLAUDE.md")).unwrap();

    assert_eq!(claude_md_after_first, claude_md_after_second);

    let work_lore_md = std::fs::read_to_string(work_dir.join("LORE.md")).unwrap();
    assert_eq!(work_lore_md.lines().filter(|l| l.trim() == "<!-- from-claude -->").count(), 1);
}

#[test]
fn sync_rewires_broken_skills_symlink() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.lore().arg("init").arg("--account").arg("work").assert().success();

    let work_skills = env.home.path().join(".claude-work/skills");
    assert!(work_skills.is_symlink());
    std::fs::remove_file(&work_skills).unwrap();
    assert!(!work_skills.exists());

    env.lore().arg("accounts").arg("sync").assert().success();

    assert!(work_skills.is_symlink());
}

#[test]
fn sync_rewires_skills_path_replaced_by_real_directory() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.lore().arg("init").arg("--account").arg("work").assert().success();

    let work_skills = env.home.path().join(".claude-work/skills");
    assert!(work_skills.is_symlink());
    std::fs::remove_file(&work_skills).unwrap();
    std::fs::create_dir(&work_skills).unwrap();
    assert!(work_skills.is_dir() && !work_skills.is_symlink());

    env.lore().arg("accounts").arg("sync").assert().success();

    assert!(work_skills.is_symlink());
}

#[test]
fn sync_rewires_claude_md_replaced_by_real_directory() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.lore().arg("init").arg("--account").arg("work").assert().success();

    let work_md = env.home.path().join(".claude-work/CLAUDE.md");
    assert!(work_md.is_file());
    std::fs::remove_file(&work_md).unwrap();
    std::fs::create_dir(&work_md).unwrap();
    assert!(work_md.is_dir());

    env.lore().arg("accounts").arg("sync").assert().success();

    assert!(work_md.is_file());
}

#[test]
fn sync_rewires_claude_md_replaced_by_symlink_to_elsewhere() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.lore().arg("init").arg("--account").arg("work").assert().success();

    let work_md = env.home.path().join(".claude-work/CLAUDE.md");
    let decoy = env.home.path().join("decoy.txt");
    std::fs::write(&decoy, "decoy content\n").unwrap();
    std::fs::remove_file(&work_md).unwrap();
    std::os::unix::fs::symlink(&decoy, &work_md).unwrap();
    assert!(work_md.is_symlink());

    env.lore().arg("accounts").arg("sync").assert().success();

    assert!(!work_md.is_symlink(), "CLAUDE.md should be a real file, not a symlink");
    let content = std::fs::read_to_string(&work_md).unwrap();
    assert!(content.starts_with('@'));
    assert_eq!(std::fs::read_to_string(&decoy).unwrap(), "decoy content\n");
}

#[test]
fn sync_recreates_deleted_lore_md() {
    let env = Env::new();
    env.lore().arg("init").arg("--account").arg("work").assert().success();

    let work_lore_md = env.home.path().join(".claude-work/LORE.md");
    std::fs::remove_file(&work_lore_md).unwrap();
    assert!(!work_lore_md.exists());

    env.lore()
        .arg("accounts")
        .arg("sync")
        .assert()
        .success()
        .stdout(predicates::str::contains("Re-wired account: work"));

    assert!(work_lore_md.is_file());
    let content = std::fs::read_to_string(&work_lore_md).unwrap();
    assert!(content.contains(&format!("@{}", env.agents_md().display())));
}

#[test]
fn sync_rewires_claude_md_missing_lore_import() {
    let env = Env::new();
    env.lore().arg("init").arg("--account").arg("work").assert().success();

    let work_md = env.home.path().join(".claude-work/CLAUDE.md");
    std::fs::write(&work_md, "").unwrap();

    env.lore().arg("accounts").arg("sync").assert().success();

    let work_lore_md = env.home.path().join(".claude-work/LORE.md");
    let content = std::fs::read_to_string(&work_md).unwrap();
    assert!(content.contains(&format!("@{}", work_lore_md.display())));
}

#[test]
fn sync_rewires_multiple_broken_accounts() {
    let env = Env::new();
    env.lore().arg("init").arg("--account").arg("work").assert().success();
    env.lore().arg("init").arg("--account").arg("personal").assert().success();

    let work_md = env.home.path().join(".claude-work/CLAUDE.md");
    let personal_md = env.home.path().join(".claude-personal/CLAUDE.md");
    std::fs::remove_file(&work_md).unwrap();
    std::fs::remove_file(&personal_md).unwrap();

    env.lore()
        .arg("accounts")
        .arg("sync")
        .assert()
        .success()
        .stdout(predicates::str::contains("work"))
        .stdout(predicates::str::contains("personal"));

    assert!(work_md.is_file());
    assert!(personal_md.is_file());
}

#[test]
fn remove_unregisters_account() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.lore().arg("init").arg("--account").arg("work").assert().success();

    env.lore().arg("accounts").arg("remove").arg("work").assert().success();

    env.lore()
        .arg("accounts")
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("work").not());
}

#[test]
fn remove_does_not_touch_disk() {
    let env = Env::new();
    env.lore().arg("init").arg("--account").arg("work").assert().success();

    let claude_md = env.home.path().join(".claude-work/CLAUDE.md");
    assert!(claude_md.is_file());

    env.lore().arg("accounts").arg("remove").arg("work").assert().success();

    assert!(claude_md.is_file());
}

#[test]
fn remove_default_warns_but_exits_0() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    env.lore()
        .arg("accounts")
        .arg("remove")
        .arg("default")
        .assert()
        .success()
        .stdout(predicates::str::contains("⚠"));

    env.lore()
        .arg("accounts")
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("default").not());
}

#[test]
fn remove_nonexistent_warns_exits_0() {
    let env = Env::new();

    env.lore()
        .arg("accounts")
        .arg("remove")
        .arg("nonexistent")
        .assert()
        .success()
        .stdout(predicates::str::contains("⚠"));
}

#[test]
fn sync_rewires_broken_account() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.lore().arg("init").arg("--account").arg("work").assert().success();

    let claude_md = env.home.path().join(".claude-work/CLAUDE.md");
    std::fs::remove_file(&claude_md).unwrap();
    assert!(!claude_md.exists());

    env.lore().arg("accounts").arg("sync").assert().success();

    assert!(claude_md.is_file());
}

#[test]
fn sync_noop_when_all_wired() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    env.lore()
        .arg("accounts")
        .arg("sync")
        .assert()
        .success()
        .stdout(predicates::str::contains("already in sync"));
}

#[test]
fn init_account_invalid_name_exits_1() {
    let env = Env::new();
    env.lore().arg("init").arg("--account").arg("bad name!").assert().failure();

    assert!(!env.home.path().join(".claude-bad name!").exists());
    assert!(!env.agents_dir.exists());
}

#[test]
fn init_account_empty_name_exits_1() {
    // Expected red — see RJ-54 test report: empty name vacuously passes validation today.
    let env = Env::new();
    env.lore().arg("init").arg("--account").arg("").assert().failure();

    assert!(!env.home.path().join(".claude-").exists());
}

#[test]
fn init_account_registers_in_config() {
    let env = Env::new();
    env.lore().arg("init").arg("--account").arg("work").assert().success();

    let config = std::fs::read_to_string(&env.config_path).unwrap();
    assert!(config.contains("work"));
    assert!(config.contains(&env.home.path().join(".claude-work").display().to_string()));
}

#[test]
fn init_account_default_is_alias_for_implicit_default() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.lore().arg("init").arg("--account").arg("default").assert().success();

    // Must stay a single registry entry pointing at the original implicit
    // default path — never a second, untracked `~/.claude-default/`.
    let config = std::fs::read_to_string(&env.config_path).unwrap();
    assert_eq!(config.lines().filter(|l| l.starts_with("default = ")).count(), 1);
    assert!(!env.home.path().join(".claude-default").exists());
}

#[test]
fn two_accounts_isolated() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.lore().arg("init").arg("--account").arg("work").assert().success();

    let work_dir = env.home.path().join(".claude-work");
    let work_lore_md = work_dir.join("LORE.md");

    let default_md = std::fs::read_to_string(env.claude_md()).unwrap();
    let work_md = std::fs::read_to_string(work_dir.join("CLAUDE.md")).unwrap();
    assert!(default_md.contains(&format!("@{}", env.lore_md().display())));
    assert!(work_md.contains(&format!("@{}", work_lore_md.display())));

    let default_lore_md = std::fs::read_to_string(env.lore_md()).unwrap();
    let work_lore_md_content = std::fs::read_to_string(&work_lore_md).unwrap();
    assert!(default_lore_md.contains(&format!("@{}", env.agents_md().display())));
    assert!(work_lore_md_content.contains(&format!("@{}", env.agents_md().display())));

    assert!(env.claude_skills().is_symlink());
    assert!(work_dir.join("skills").is_symlink());
}
