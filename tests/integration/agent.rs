use crate::helpers::{Env, make_agent};
use std::fs;

fn add(env: &Env, cwd: &std::path::Path, args: &[&str]) -> assert_cmd::assert::Assert {
    env.lore()
        .arg("agent")
        .arg("add")
        .args(args)
        .current_dir(cwd)
        .assert()
}

fn remove(env: &Env, args: &[&str]) -> assert_cmd::assert::Assert {
    env.lore().arg("agent").arg("remove").args(args).assert()
}

fn shared(env: &Env, file: &str) -> std::path::PathBuf {
    env.agents_dir.join("agents").join(file)
}

#[test]
fn shared_add_creates_link_to_cwd_file() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    let src = env.home.path().join("src");
    make_agent(&src, "reviewer");

    add(&env, &src, &["reviewer"]).success();

    let link = shared(&env, "reviewer.md");
    assert!(link.is_symlink());
    assert_eq!(
        fs::read_link(&link).unwrap(),
        src.canonicalize().unwrap().join("reviewer.md")
    );
}

#[test]
fn shared_add_relinks_into_every_registered_account_including_default() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");
    let src = env.home.path().join("src");
    make_agent(&src, "reviewer");

    add(&env, &src, &["reviewer"]).success();

    for link in [
        env.claude_agents().join("reviewer.md"),
        env.account_agents("work").join("reviewer.md"),
    ] {
        assert!(link.is_symlink(), "{} should be a link", link.display());
        assert_eq!(fs::read_link(&link).unwrap(), shared(&env, "reviewer.md"));
    }
}

#[test]
fn add_with_trailing_md_behaves_like_without() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    let src = env.home.path().join("src");
    make_agent(&src, "reviewer");

    add(&env, &src, &["reviewer.md"]).success();

    assert!(shared(&env, "reviewer.md").is_symlink());
    assert!(!shared(&env, "reviewer.md.md").exists());
}

#[test]
fn add_with_missing_source_warns_and_still_installs_the_rest() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    let src = env.home.path().join("src");
    make_agent(&src, "reviewer");

    add(&env, &src, &["ghost", "reviewer"])
        .success()
        .stdout(predicates::str::contains("'ghost.md' not found"));

    assert!(!shared(&env, "ghost.md").exists());
    assert!(shared(&env, "reviewer.md").is_symlink());
}

#[test]
fn duplicate_add_warns_and_does_not_overwrite() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    let first = env.home.path().join("first");
    let second = env.home.path().join("second");
    make_agent(&first, "reviewer");
    make_agent(&second, "reviewer");

    add(&env, &first, &["reviewer"]).success();
    add(&env, &second, &["reviewer"])
        .success()
        .stdout(predicates::str::contains("already installed"))
        .stdout(predicates::str::contains("existing"))
        .stdout(predicates::str::contains("attempted"));

    assert_eq!(
        fs::read_link(shared(&env, "reviewer.md")).unwrap(),
        first.canonicalize().unwrap().join("reviewer.md")
    );
}

#[test]
fn scoped_add_links_into_that_account_only() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");
    env.register_account("other");
    let src = env.home.path().join("src");
    make_agent(&src, "reviewer");

    add(&env, &src, &["--account", "work", "reviewer"]).success();

    let link = env.account_agents("work").join("reviewer.md");
    assert_eq!(
        fs::read_link(&link).unwrap(),
        src.canonicalize().unwrap().join("reviewer.md")
    );
    assert!(!shared(&env, "reviewer.md").exists());
    assert!(!env.account_agents("other").join("reviewer.md").exists());
    assert!(!env.claude_agents().join("reviewer.md").exists());
}

#[test]
fn account_default_scopes_to_default_claude_dir_only() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");
    let src = env.home.path().join("src");
    make_agent(&src, "reviewer");

    add(&env, &src, &["--account", "default", "reviewer"]).success();

    assert!(env.claude_agents().join("reviewer.md").is_symlink());
    assert!(!shared(&env, "reviewer.md").exists());
    assert!(!env.account_agents("work").join("reviewer.md").exists());
}

#[test]
fn scoped_add_to_unregistered_account_fails_and_creates_nothing() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    let src = env.home.path().join("src");
    make_agent(&src, "reviewer");

    add(&env, &src, &["--account", "nope", "reviewer"]).failure();

    assert!(!env.account_agents("nope").exists());
    assert!(!shared(&env, "reviewer.md").exists());
}

#[test]
fn shared_add_skips_account_holding_a_real_file_of_the_same_name() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");
    let real = env.account_agents("work").join("reviewer.md");
    fs::write(&real, "hand-written").unwrap();
    let src = env.home.path().join("src");
    make_agent(&src, "reviewer");

    add(&env, &src, &["reviewer"])
        .success()
        .stdout(predicates::str::contains("is not a symlink"));

    assert!(!real.is_symlink());
    assert_eq!(fs::read_to_string(&real).unwrap(), "hand-written");
    assert!(env.claude_agents().join("reviewer.md").is_symlink());
}

#[test]
fn shared_remove_deletes_links_and_keeps_source() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");
    let src = env.home.path().join("src");
    let source_file = make_agent(&src, "reviewer");
    add(&env, &src, &["reviewer"]).success();

    remove(&env, &["reviewer"]).success();

    assert!(!shared(&env, "reviewer.md").is_symlink());
    assert!(!env.claude_agents().join("reviewer.md").is_symlink());
    assert!(!env.account_agents("work").join("reviewer.md").is_symlink());
    assert!(source_file.is_file());
}

#[test]
fn shared_remove_preserves_differently_sourced_scoped_link() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");
    let shared_src = env.home.path().join("shared-src");
    let scoped_src = env.home.path().join("scoped-src");
    make_agent(&shared_src, "reviewer");
    make_agent(&scoped_src, "reviewer");
    add(&env, &scoped_src, &["--account", "work", "reviewer"]).success();
    add(&env, &shared_src, &["reviewer"]).success();

    remove(&env, &["reviewer"])
        .success()
        .stdout(predicates::str::contains("points elsewhere"));

    assert!(!shared(&env, "reviewer.md").is_symlink());
    assert_eq!(
        fs::read_link(env.account_agents("work").join("reviewer.md")).unwrap(),
        scoped_src.canonicalize().unwrap().join("reviewer.md")
    );
}

#[test]
fn scoped_remove_only_affects_targeted_account() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");
    let src = env.home.path().join("src");
    make_agent(&src, "reviewer");
    add(&env, &src, &["reviewer"]).success();

    remove(&env, &["--account", "work", "reviewer"]).success();

    assert!(!env.account_agents("work").join("reviewer.md").is_symlink());
    assert!(shared(&env, "reviewer.md").is_symlink());
    assert!(env.claude_agents().join("reviewer.md").is_symlink());
}

#[test]
fn remove_of_absent_name_warns_and_exits_0() {
    let env = Env::new();
    env.lore().arg("init").assert().success();

    remove(&env, &["ghost"])
        .success()
        .stdout(predicates::str::contains("ghost is not installed"));
}

fn symlink_account_agents_dir(env: &Env, account: &str, target: &std::path::Path) {
    let dir = env.account_agents(account);
    fs::remove_dir_all(&dir).unwrap();
    std::os::unix::fs::symlink(target, &dir).unwrap();
}

#[test]
fn scoped_add_refuses_symlinked_account_agents_dir_aliasing_the_shared_pool() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");
    symlink_account_agents_dir(&env, "work", &env.agents_dir.join("agents"));
    let src = env.home.path().join("src");
    make_agent(&src, "reviewer");

    add(&env, &src, &["--account", "work", "reviewer"])
        .failure()
        .stderr(predicates::str::contains("is a symlink"));

    assert!(!shared(&env, "reviewer.md").exists());
    assert!(!env.claude_agents().join("reviewer.md").exists());
}

#[test]
fn scoped_remove_refuses_symlinked_account_agents_dir_and_keeps_shared_link() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");
    let src = env.home.path().join("src");
    make_agent(&src, "reviewer");
    add(&env, &src, &["reviewer"]).success();
    symlink_account_agents_dir(&env, "work", &env.agents_dir.join("agents"));

    remove(&env, &["--account", "work", "reviewer"])
        .failure()
        .stderr(predicates::str::contains("is a symlink"));

    assert!(shared(&env, "reviewer.md").is_symlink());
    assert!(env.claude_agents().join("reviewer.md").is_symlink());
}

#[test]
fn shared_add_skips_symlinked_account_agents_dir_and_warns() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");
    let user_dir = env.home.path().join("user-agents");
    fs::create_dir_all(&user_dir).unwrap();
    symlink_account_agents_dir(&env, "work", &user_dir);
    let src = env.home.path().join("src");
    make_agent(&src, "reviewer");

    add(&env, &src, &["reviewer"])
        .success()
        .stdout(predicates::str::contains("is a symlink"));

    assert_eq!(fs::read_dir(&user_dir).unwrap().count(), 0);
    assert!(shared(&env, "reviewer.md").is_symlink());
    assert!(env.claude_agents().join("reviewer.md").is_symlink());
}

#[test]
fn shared_remove_skips_symlinked_account_agents_dir_and_warns() {
    let env = Env::new();
    env.lore().arg("init").assert().success();
    env.register_account("work");
    let user_dir = env.home.path().join("user-agents");
    fs::create_dir_all(&user_dir).unwrap();
    let kept = user_dir.join("reviewer.md");
    fs::write(&kept, "user's own").unwrap();
    symlink_account_agents_dir(&env, "work", &user_dir);
    let src = env.home.path().join("src");
    make_agent(&src, "reviewer");
    add(&env, &src, &["reviewer"]).success();

    remove(&env, &["reviewer"])
        .success()
        .stdout(predicates::str::contains("is a symlink"));

    assert_eq!(fs::read_to_string(&kept).unwrap(), "user's own");
    assert!(!shared(&env, "reviewer.md").is_symlink());
}
