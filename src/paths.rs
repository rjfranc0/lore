use std::path::PathBuf;

use anyhow::Result;

use crate::config::LoreConfig;

pub struct Paths {
    pub agents_dir: PathBuf,
    pub skills_dir: PathBuf,
    pub behaviors_dir: PathBuf,
    pub agents_md: PathBuf,
}

impl Paths {
    pub fn load() -> Result<Self> {
        let config_path = LoreConfig::config_path();
        let config = LoreConfig::load_or_default(&config_path)?;
        Ok(Self::from_config(&config))
    }

    pub fn from_config(config: &LoreConfig) -> Self {
        let agents_dir = config.agents_dir_path();
        let skills_dir = agents_dir.join("skills");
        let behaviors_dir = agents_dir.join("behaviors");
        let agents_md = agents_dir.join("AGENTS.md");

        Self { agents_dir, skills_dir, behaviors_dir, agents_md }
    }
}
