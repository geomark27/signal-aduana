//! Local cache: the last full board load, to show it on open and without a network.
//! On Windows it lives at `%LOCALAPPDATA%\ec.com.torresytorres.signal\cache.json`.
//!
//! `data` is opaque to Rust: the UI decides what it stores. The file never contains the token
//! or the email; only the `account_id` and the site, to avoid showing another account's data.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AppError, AppResult};

/// File format version. If it changes, earlier caches are ignored.
const VERSION: u32 = 1;

/// Maximum size of the serialized file: 20 MB.
pub const MAX_BYTES: usize = 20 * 1024 * 1024;

/// What is returned to the UI (Contract C).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// RFC 3339 date-time (UTC) at which the file was written.
    pub saved_at: String,
    pub data: Value,
}

#[derive(Serialize, Deserialize)]
struct CacheFile {
    version: u32,
    account_id: String,
    site: String,
    saved_at: String,
    data: Value,
}

pub fn path(dir: &Path, simulated: bool) -> PathBuf {
    dir.join(if simulated { "cache.simulated.json" } else { "cache.json" })
}

/// Saves `data` atomically (temp file + rename).
pub fn save(dir: &Path, simulated: bool, account_id: &str, site: &str, data: Value) -> AppResult<()> {
    let file = CacheFile {
        version: VERSION,
        account_id: account_id.to_string(),
        site: site.to_string(),
        saved_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        data,
    };
    let text = serde_json::to_vec(&file).map_err(|e| AppError::Cache(e.to_string()))?;
    if text.len() > MAX_BYTES {
        return Err(AppError::Cache(format!(
            "Los datos guardados superan el tamaño máximo de {} MB.",
            MAX_BYTES / (1024 * 1024)
        )));
    }
    fs::create_dir_all(dir).map_err(|e| AppError::Cache(e.to_string()))?;
    let target = path(dir, simulated);
    let tmp = target.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|e| AppError::Cache(e.to_string()))?;
    fs::rename(&tmp, &target).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        AppError::Cache(e.to_string())
    })
}

/// `None` if there is no file, it is corrupt, it has another version or it belongs to another account or site.
pub fn read(dir: &Path, simulated: bool, account_id: &str, site: &str) -> Option<Snapshot> {
    let bytes = fs::read(path(dir, simulated)).ok()?;
    let f: CacheFile = serde_json::from_slice(&bytes).ok()?;
    if f.version != VERSION || f.account_id != account_id || f.site != site {
        return None;
    }
    Some(Snapshot { saved_at: f.saved_at, data: f.data })
}

/// Deletes the cache; it not existing is not an error.
pub fn delete(dir: &Path, simulated: bool) {
    let _ = fs::remove_file(path(dir, simulated));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SITE: &str = "https://x.atlassian.net";

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read(dir.path(), false, "u1", SITE).is_none());
        let data = json!({ "format": 1, "issues": [{ "key": "APP-1" }] });
        save(dir.path(), false, "u1", SITE, data.clone()).unwrap();
        let s = read(dir.path(), false, "u1", SITE).unwrap();
        assert_eq!(s.data, data);
        assert!(chrono::DateTime::parse_from_rfc3339(&s.saved_at).is_ok());
        assert!(s.saved_at.ends_with('Z'));
        let text = fs::read_to_string(path(dir.path(), false)).unwrap();
        assert!(!text.contains("token") && !text.contains('@'));
    }

    #[test]
    fn simulated_uses_another_file() {
        let dir = tempfile::tempdir().unwrap();
        save(dir.path(), true, "u1", SITE, json!(1)).unwrap();
        assert!(path(dir.path(), true).ends_with("cache.simulated.json"));
        assert!(read(dir.path(), true, "u1", SITE).is_some());
        assert!(read(dir.path(), false, "u1", SITE).is_none());
    }

    #[test]
    fn other_account_or_site_gives_none() {
        let dir = tempfile::tempdir().unwrap();
        save(dir.path(), false, "u1", SITE, json!({ "a": 1 })).unwrap();
        assert!(read(dir.path(), false, "u2", SITE).is_none());
        assert!(read(dir.path(), false, "u1", "https://y.atlassian.net").is_none());
        assert!(read(dir.path(), false, "u1", SITE).is_some());
    }

    #[test]
    fn corrupt_file_gives_none() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(path(dir.path(), false), "esto no es json {{{").unwrap();
        assert!(read(dir.path(), false, "u1", SITE).is_none());
        fs::write(path(dir.path(), false), r#"{"version":1}"#).unwrap();
        assert!(read(dir.path(), false, "u1", SITE).is_none());
    }

    #[test]
    fn different_version_gives_none() {
        let dir = tempfile::tempdir().unwrap();
        let other = json!({ "version": 2, "account_id": "u1", "site": SITE,
            "saved_at": "2026-01-01T00:00:00Z", "data": {} });
        fs::write(path(dir.path(), false), other.to_string()).unwrap();
        assert!(read(dir.path(), false, "u1", SITE).is_none());
    }

    #[test]
    fn rejects_data_that_is_too_large() {
        let dir = tempfile::tempdir().unwrap();
        let large = Value::String("a".repeat(MAX_BYTES + 1));
        let r = save(dir.path(), false, "u1", SITE, large);
        assert!(matches!(r, Err(AppError::Cache(_))));
        assert!(!path(dir.path(), false).exists());
    }

    #[test]
    fn delete_does_not_fail_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        delete(dir.path(), false);
        save(dir.path(), false, "u1", SITE, json!(1)).unwrap();
        delete(dir.path(), false);
        assert!(read(dir.path(), false, "u1", SITE).is_none());
    }

    #[test]
    fn creates_the_directory_and_leaves_no_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("new").join("nested");
        save(&sub, false, "u1", SITE, json!(1)).unwrap();
        let names: Vec<_> = fs::read_dir(&sub).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names, vec![std::ffi::OsString::from("cache.json")]);
    }
}
