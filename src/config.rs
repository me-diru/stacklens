use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Folders to search for git repositories. `~` is expanded.
    pub roots: Vec<String>,
    pub max_depth: usize,
    pub collectors: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            roots: vec!["~".into()],
            max_depth: 2,
            collectors: crate::collectors::ORDER
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

impl Config {
    /// Looks for stacklens.config.json in `dir`, then in the user config dir.
    pub fn load(dir: &Path) -> Self {
        let candidates = [
            dir.join("stacklens.config.json"),
            dirs::config_dir()
                .unwrap_or_default()
                .join("stacklens")
                .join("config.json"),
        ];
        for p in candidates {
            if let Ok(txt) = std::fs::read_to_string(&p) {
                if let Ok(c) = serde_json::from_str(&txt) {
                    return c;
                }
            }
        }
        Self::default()
    }
}
