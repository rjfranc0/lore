use std::fs;
use crate::helpers::Env;

#[test]
fn creates_expected_structure() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    assert!(env.agents_dir.join("skills").is_dir());
    assert!(env.agents_dir.join("behaviors").is_dir());
    assert!(env.agents_md().is_file());
    assert!(env.claude_md().is_file());
    assert!(env.claude_skills().is_symlink());

    let claude_md = fs::read_to_string(env.claude_md()).unwrap();
    assert!(claude_md.contains(&format!("@{}", env.agents_md().display())));
}

#[test]
fn is_idempotent() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.lore().arg("init").assert().success();
    assert!(env.claude_skills().is_symlink());
}

#[test]
fn migrates_existing_claude_md() {
    let env = Env::new();
    fs::write(env.claude_md(), "# old rules\nbe nice\n").unwrap();
    env.lore().arg("init").assert().success();

    let rules = env.agents_dir.join("behaviors/from-claude/RULES.md");
    assert!(rules.is_file());
    assert!(fs::read_to_string(&rules).unwrap().contains("old rules"));

    let claude_md = fs::read_to_string(env.claude_md()).unwrap();
    assert!(claude_md.contains(&format!("@{}", env.agents_md().display())));

    let agents_md = fs::read_to_string(env.agents_md()).unwrap();
    assert!(agents_md.contains("from-claude"));
}

#[test]
fn safe_fail_on_skill_collision_claude_md_not_written() {
    let env = Env::new();
    // Put a real skills dir with a skill that also exists in agents
    let claude_skills = env.claude_skills();
    fs::create_dir_all(claude_skills.join("dup")).unwrap();
    fs::write(claude_skills.join("dup/SKILL.md"), "").unwrap();
    fs::create_dir_all(env.agents_dir.join("skills/dup")).unwrap();
    fs::write(env.agents_dir.join("skills/dup/SKILL.md"), "").unwrap();

    // AGENTS.md must exist so we don't enter migration path
    fs::create_dir_all(&env.agents_dir).unwrap();
    fs::create_dir_all(env.agents_dir.join("behaviors")).unwrap();
    fs::write(env.agents_md(), "<!-- managed by lore -->\n").unwrap();

    // Point CLAUDE.md at agents_md so init skips migration
    fs::write(env.claude_md(), format!("@{}\n", env.agents_md().display())).unwrap();

    env.lore().arg("init").assert().failure();

    // CLAUDE.md must still point to agents_md, not be overwritten
    let content = fs::read_to_string(env.claude_md()).unwrap();
    assert!(content.contains(&format!("@{}", env.agents_md().display())));
    // skills symlink must not have been created
    assert!(!env.claude_skills().is_symlink());
}

#[test]
fn recovery_reregisters_existing_behaviors_when_agents_md_deleted() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    // Add a behavior
    let bsrc = env.home.path().join("src-behaviors");
    crate::helpers::make_behavior(&bsrc, "my-rules", "RULES.md");
    env.lore().arg("behavior").arg("add").arg("my-rules")
        .current_dir(&bsrc).assert().success();

    let agents_md = fs::read_to_string(env.agents_md()).unwrap();
    assert!(agents_md.contains("<!-- my-rules -->"));

    // Delete AGENTS.md
    fs::remove_file(env.agents_md()).unwrap();

    env.lore().arg("init").assert().success();

    let agents_md = fs::read_to_string(env.agents_md()).unwrap();
    assert!(agents_md.contains("<!-- my-rules -->"));
}
