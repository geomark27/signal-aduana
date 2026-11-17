//! Non-secret configuration: site and email of the last session.
//! On Windows it lives at `%APPDATA%\ec.com.torresytorres.signal\config.toml`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub site: Option<String>,
    pub email: Option<String>,
}

pub fn path(dir: &Path, simulated: bool) -> PathBuf {
    dir.join(if simulated { "config.simulated.toml" } else { "config.toml" })
}

impl Config {
    /// If the file does not exist or is corrupt, it starts with an empty configuration.
    pub fn load(path: &Path) -> Config {
        fs::read_to_string(path)
            .ok()
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> AppResult<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| AppError::Config(e.to_string()))?;
        }
        let text = toml::to_string_pretty(self).map_err(|e| AppError::Config(e.to_string()))?;
        // Written to a temp file and then renamed, so the file is never left half-written.
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, text).map_err(|e| AppError::Config(e.to_string()))?;
        fs::rename(&tmp, path).map_err(|e| AppError::Config(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_loads() {
        let dir = tempfile::tempdir().unwrap();
        let p = path(dir.path(), false);
        assert_eq!(Config::load(&p), Config::default());
        let c = Config {
            site: Some("https://x.atlassian.net".into()),
            email: Some("a@b.com".into()),
        };
        c.save(&p).unwrap();
        assert_eq!(Config::load(&p), c);
        let text = fs::read_to_string(&p).unwrap();
        assert!(!text.contains("token"));
    }

    #[test]
    fn corrupt_file_does_not_break() {
        let dir = tempfile::tempdir().unwrap();
        let p = path(dir.path(), true);
        fs::write(&p, "esto no es toml [[[").unwrap();
        assert_eq!(Config::load(&p), Config::default());
    }
}
