use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub struct Env {
    pub home: TempDir,
    pub agents_dir: PathBuf,
    pub claude_dir: PathBuf,
}

impl Env {
    pub fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let agents_dir = home.path().join(".agents");
        let claude_dir = home.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        Self { home, agents_dir, claude_dir }
    }

    pub fn lore(&self) -> assert_cmd::Command {
        let mut cmd = assert_cmd::Command::cargo_bin("lore").unwrap();
        cmd.env("AGENTS_DIR", &self.agents_dir)
           .env("CLAUDE_DIR", &self.claude_dir)
           .env("PAGER", "cat");
        cmd
    }

    pub fn agents_md(&self) -> PathBuf {
        self.agents_dir.join("AGENTS.md")
    }

    pub fn claude_md(&self) -> PathBuf {
        self.claude_dir.join("CLAUDE.md")
    }

    pub fn claude_skills(&self) -> PathBuf {
        self.claude_dir.join("skills")
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
