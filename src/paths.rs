use std::path::PathBuf;

pub struct Paths {
    pub agents_dir: PathBuf,
    pub skills_dir: PathBuf,
    pub behaviors_dir: PathBuf,
    pub agents_md: PathBuf,
    pub claude_dir: PathBuf,
}

impl Paths {
    pub fn resolve() -> Self {
        let agents_dir = std::env::var("AGENTS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .expect("cannot determine home directory")
                    .join(".agents")
            });

        let claude_dir = std::env::var("CLAUDE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .expect("cannot determine home directory")
                    .join(".claude")
            });

        let skills_dir = agents_dir.join("skills");
        let behaviors_dir = agents_dir.join("behaviors");
        let agents_md = agents_dir.join("AGENTS.md");

        Self { agents_dir, skills_dir, behaviors_dir, agents_md, claude_dir }
    }
}
