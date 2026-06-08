use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

pub struct AgentsMd {
    pub header: String,
    pub behaviors: Vec<BehaviorBlock>,
}

pub struct BehaviorBlock {
    pub name: String,
    pub path: PathBuf,
}

impl AgentsMd {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        Ok(Self::parse(&content))
    }

    pub fn parse(content: &str) -> Self {
        let mut header_lines: Vec<&str> = Vec::new();
        let mut behaviors: Vec<BehaviorBlock> = Vec::new();
        let mut lines = content.lines().peekable();

        // Collect header: everything before the first <!-- name --> block
        // A block starts with a line matching exactly `<!-- <name> -->`
        while let Some(&line) = lines.peek() {
            if is_block_comment(line) {
                break;
            }
            header_lines.push(line);
            lines.next();
        }

        // Strip trailing blank lines from header, keep one trailing newline
        while header_lines.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
            header_lines.pop();
        }
        let header = if header_lines.is_empty() {
            String::new()
        } else {
            header_lines.join("\n") + "\n"
        };

        // Parse blocks
        while let Some(line) = lines.next() {
            if let Some(name) = parse_block_comment(line) {
                if let Some(at_line) = lines.next() {
                    if let Some(path_str) = at_line.strip_prefix('@') {
                        behaviors.push(BehaviorBlock {
                            name: name.to_string(),
                            path: PathBuf::from(path_str),
                        });
                    }
                }
            }
        }

        Self { header, behaviors }
    }

    pub fn serialize(&self) -> String {
        let mut out = self.header.clone();
        for b in &self.behaviors {
            out.push('\n');
            out.push_str(&format!("<!-- {} -->\n", b.name));
            out.push_str(&format!("@{}\n", b.path.display()));
        }
        out
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, self.serialize())
            .with_context(|| format!("cannot write {}", path.display()))
    }

    pub fn contains_path(&self, path: &Path) -> bool {
        self.behaviors.iter().any(|b| b.path == path)
    }

    pub fn contains_name(&self, name: &str) -> bool {
        self.behaviors.iter().any(|b| b.name == name)
    }

    pub fn add(&mut self, name: String, path: PathBuf) {
        self.behaviors.push(BehaviorBlock { name, path });
    }

    pub fn remove_by_name(&mut self, name: &str) {
        self.behaviors.retain(|b| b.name != name);
    }
}

fn is_block_comment(line: &str) -> bool {
    parse_block_comment(line).is_some()
}

fn parse_block_comment(line: &str) -> Option<&str> {
    let s = line.strip_prefix("<!-- ")?.strip_suffix(" -->")?;
    // name must be non-empty and contain no spaces
    if s.is_empty() || s.contains(' ') {
        return None;
    }
    Some(s)
}

/// Find the entry .md file inside a behavior directory.
/// Resolution order: RULES.md → BEHAVIOR.md → README.md → first .md (sorted)
pub fn behavior_entry(dir: &Path) -> Result<PathBuf> {
    for name in &["RULES.md", "BEHAVIOR.md", "README.md"] {
        let p = dir.join(name);
        if p.is_file() {
            return Ok(p);
        }
    }
    // First .md alphabetically
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("md") && p.is_file())
        .collect();
    candidates.sort();
    candidates.into_iter().next()
        .ok_or_else(|| anyhow::anyhow!("no .md entry point found in {}", dir.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
<!-- managed by lore — do not edit -->
<!-- skills auto-loaded from ~/.agents/skills/ -->

<!-- from-claude -->
@/home/you/.agents/behaviors/from-claude/RULES.md

<!-- restaurant-rules -->
@/home/you/.agents/behaviors/restaurant-rules/RULES.md
";

    #[test]
    fn round_trip() {
        let md = AgentsMd::parse(SAMPLE);
        assert_eq!(md.serialize(), SAMPLE);
    }

    #[test]
    fn header_preserved() {
        let md = AgentsMd::parse(SAMPLE);
        assert!(md.header.contains("managed by lore"));
    }

    #[test]
    fn behaviors_parsed() {
        let md = AgentsMd::parse(SAMPLE);
        assert_eq!(md.behaviors.len(), 2);
        assert_eq!(md.behaviors[0].name, "from-claude");
        assert_eq!(md.behaviors[1].name, "restaurant-rules");
    }

    #[test]
    fn exact_name_match_no_clobber() {
        let content = "\
<!-- managed by lore — do not edit -->\n\
\n\
<!-- a.c -->\n\
@/path/a.c/RULES.md\n\
\n\
<!-- axc -->\n\
@/path/axc/RULES.md\n";
        let mut md = AgentsMd::parse(content);
        md.remove_by_name("a.c");
        assert!(!md.contains_name("a.c"));
        assert!(md.contains_name("axc"));
    }

    #[test]
    fn add_and_remove() {
        let mut md = AgentsMd::parse(SAMPLE);
        md.add("new-rule".into(), PathBuf::from("/path/new-rule/RULES.md"));
        assert!(md.contains_name("new-rule"));
        md.remove_by_name("new-rule");
        assert!(!md.contains_name("new-rule"));
    }
}
