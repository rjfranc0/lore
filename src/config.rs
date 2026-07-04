use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct LoreConfig {
    pub agents_dir: String,
    #[serde(default)]
    pub accounts: BTreeMap<String, String>,
}

impl Default for LoreConfig {
    fn default() -> Self {
        let agents_dir = dirs::home_dir()
            .expect("cannot determine home directory")
            .join(".agents")
            .to_string_lossy()
            .into_owned();
        Self { agents_dir, accounts: BTreeMap::new() }
    }
}

impl LoreConfig {
    pub fn config_path() -> PathBuf {
        std::env::var("LORE_CONF").map(PathBuf::from).unwrap_or_else(|_| {
            dirs::home_dir()
                .expect("cannot determine home directory")
                .join(".config/lore/lore.toml")
        })
    }

    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        toml::from_str(&content).with_context(|| format!("cannot parse {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self).context("cannot serialize lore config")?;
        std::fs::write(path, content).with_context(|| format!("cannot write {}", path.display()))
    }

    pub fn agents_dir_path(&self) -> PathBuf {
        PathBuf::from(&self.agents_dir)
    }

    pub fn account_path(&self, name: &str) -> Option<PathBuf> {
        self.accounts.get(name).map(PathBuf::from)
    }

    pub fn require_account_path(&self, name: &str) -> Result<PathBuf> {
        self.account_path(name).ok_or_else(|| {
            anyhow::anyhow!(
                "account '{name}' is not registered — run `lore init --account {name}` first"
            )
        })
    }
}

pub fn validate_account_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("invalid account name: must not be empty");
    }
    if let Some(bad) = name.chars().find(|c| !c.is_ascii_alphanumeric() && *c != '-') {
        anyhow::bail!(
            "invalid account name '{name}': only alphanumeric characters and hyphens are allowed (found '{bad}')"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_or_default_returns_defaults_when_file_absent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does-not-exist/lore.toml");
        let config = LoreConfig::load_or_default(&path).unwrap();
        assert!(config.accounts.is_empty());
        assert!(config.agents_dir.ends_with(".agents"));
    }

    #[test]
    fn load_or_default_parses_valid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lore.toml");
        std::fs::write(&path, "agents_dir = \"/tmp/agents\"\n\n[accounts]\ndefault = \"/tmp/claude\"\n").unwrap();

        let config = LoreConfig::load_or_default(&path).unwrap();
        assert_eq!(config.agents_dir, "/tmp/agents");
        assert_eq!(config.account_path("default"), Some(PathBuf::from("/tmp/claude")));
    }

    #[test]
    fn save_round_trips_without_data_loss() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lore.toml");

        let mut config = LoreConfig::default();
        config.accounts.insert("work".into(), "/tmp/claude-work".into());
        config.save(&path).unwrap();

        let loaded = LoreConfig::load_or_default(&path).unwrap();
        assert_eq!(loaded.agents_dir, config.agents_dir);
        assert_eq!(loaded.accounts, config.accounts);
    }

    #[test]
    fn config_path_honors_lore_conf_env_var() {
        // SAFETY: test runs single-threaded w.r.t. this env var via serial assertions below;
        // we restore the prior value immediately after reading the result.
        let prev = std::env::var("LORE_CONF").ok();
        unsafe { std::env::set_var("LORE_CONF", "/tmp/custom-lore.toml") };
        let path = LoreConfig::config_path();
        match prev {
            Some(v) => unsafe { std::env::set_var("LORE_CONF", v) },
            None => unsafe { std::env::remove_var("LORE_CONF") },
        }
        assert_eq!(path, PathBuf::from("/tmp/custom-lore.toml"));
    }
}
