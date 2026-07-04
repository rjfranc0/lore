use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub struct Env {
    pub home: TempDir,
    pub agents_dir: PathBuf,
    pub claude_dir: PathBuf,
    pub config_path: PathBuf,
}

impl Env {
    pub fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let agents_dir = home.path().join(".agents");
        let claude_dir = home.path().join(".claude");
        let config_dir = home.path().join(".config/lore");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();

        let config_path = config_dir.join("lore.toml");
        std::fs::write(
            &config_path,
            format!(
                "agents_dir = \"{}\"\n\n[accounts]\ndefault = \"{}\"\n",
                agents_dir.display(),
                claude_dir.display(),
            ),
        )
        .unwrap();

        Self { home, agents_dir, claude_dir, config_path }
    }

    /// Like `new()`, but skips writing `lore.toml` — for tests proving true
    /// first-run bootstrap behavior (no config file on disk at all yet).
    pub fn bare() -> Self {
        let home = tempfile::tempdir().unwrap();
        let agents_dir = home.path().join(".agents");
        let claude_dir = home.path().join(".claude");
        let config_path = home.path().join(".config/lore/lore.toml");

        Self { home, agents_dir, claude_dir, config_path }
    }

    pub fn lore(&self) -> assert_cmd::Command {
        let mut cmd = assert_cmd::Command::cargo_bin("lore").unwrap();
        // `--account` resolution falls back to `dirs::home_dir()`, which reads $HOME —
        // sandbox it too, or named-account tests collide on the real home directory.
        cmd.env("HOME", self.home.path())
            .env("LORE_CONF", &self.config_path)
            .env("PAGER", "cat");
        cmd
    }

    pub fn agents_md(&self) -> PathBuf {
        self.agents_dir.join("AGENTS.md")
    }

    pub fn claude_md(&self) -> PathBuf {
        self.claude_dir.join("CLAUDE.md")
    }

    pub fn lore_md(&self) -> PathBuf {
        self.claude_dir.join("LORE.md")
    }

    pub fn claude_skills(&self) -> PathBuf {
        self.claude_dir.join("skills")
    }

    /// Registers a named account via `lore init --account <name>`.
    pub fn register_account(&self, name: &str) {
        self.lore().arg("init").arg("--account").arg(name).assert().success();
    }

    pub fn account_skills(&self, name: &str) -> PathBuf {
        self.home.path().join(format!(".claude-{name}/skills"))
    }
}

pub fn make_skill(base: &Path, name: &str) -> PathBuf {
    let dir = base.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SKILL.md"), "").unwrap();
    dir
}

pub fn make_behavior(base: &Path, name: &str, entry: &str) -> PathBuf {
    let dir = base.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(entry), "rules\n").unwrap();
    dir
}
