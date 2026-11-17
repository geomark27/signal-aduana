//! User preferences, stored on the machine (not in the browser).
//! On Windows they live in `%APPDATA%\ec.com.torresytorres.signal\preferences.toml`.
//! They do not depend on the account: they are not deleted on sign-out.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::calendar::{validate_date, ExtraHolidays};
use crate::error::{AppError, AppResult};

pub const DEFAULT_END_OF_DAY_TIME: &str = "16:30";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NotificationPrefs {
    pub enabled: bool,
    /// `HH:MM`, Ecuador time.
    pub end_of_day_time: String,
    pub weekly_summary: bool,
}

impl Default for NotificationPrefs {
    fn default() -> Self {
        NotificationPrefs { enabled: true, end_of_day_time: DEFAULT_END_OF_DAY_TIME.into(), weekly_summary: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferences {
    pub notifications: NotificationPrefs,
    pub start_with_windows: bool,
    pub minimize_to_tray: bool,
    pub extra_holidays: ExtraHolidays,
}

impl Default for Preferences {
    fn default() -> Self {
        Preferences {
            notifications: NotificationPrefs::default(),
            start_with_windows: false,
            minimize_to_tray: true,
            extra_holidays: ExtraHolidays::new(),
        }
    }
}

pub fn path(dir: &Path, simulated: bool) -> PathBuf {
    dir.join(if simulated { "preferences.simulated.toml" } else { "preferences.toml" })
}

/// Validates `HH:MM` (00:00 to 23:59, two digits each part) and returns it unchanged.
fn validate_time(time: &str) -> AppResult<String> {
    // No indexing or slicing: split into `HH` and `MM`, each part must be two digits.
    let two_digits = |t: &str| t.chars().count() == 2 && t.chars().all(|c| c.is_ascii_digit());
    let valid = time.split_once(':').is_some_and(|(h, m)| {
        two_digits(h)
            && two_digits(m)
            && h.parse::<u32>().is_ok_and(|h| h < 24)
            && m.parse::<u32>().is_ok_and(|m| m < 60)
    });
    if valid {
        Ok(time.to_string())
    } else {
        Err(AppError::Format(format!(
            "La hora de cierre debe ser HH:MM entre 00:00 y 23:59: «{}»",
            time.chars().take(20).collect::<String>()
        )))
    }
}

impl Preferences {
    /// Invalid time → `Format`; holidays with an invalid date are dropped.
    pub fn normalize(mut self) -> AppResult<Preferences> {
        self.notifications.end_of_day_time = validate_time(self.notifications.end_of_day_time.trim())?;
        self.extra_holidays = self
            .extra_holidays
            .into_iter()
            .filter_map(|(k, v)| validate_date(&k).ok().map(|d| (d, v)))
            .collect();
        Ok(self)
    }

    /// Does anything the monitor uses change (notifications or holidays)?
    pub fn affects_monitor(&self, other: &Preferences) -> bool {
        self.notifications != other.notifications || self.extra_holidays != other.extra_holidays
    }
}

/// No file or corrupt → defaults. If only the saved time is invalid, that one is reset.
pub fn load(dir: &Path, simulated: bool) -> Preferences {
    let read: Option<Preferences> =
        fs::read_to_string(path(dir, simulated)).ok().and_then(|t| toml::from_str(&t).ok());
    let Some(mut p) = read else { return Preferences::default() };
    if validate_time(p.notifications.end_of_day_time.trim()).is_err() {
        p.notifications.end_of_day_time = DEFAULT_END_OF_DAY_TIME.into();
    }
    p.normalize().unwrap_or_default()
}

/// Normalizes and saves (atomic write). Returns what was saved.
pub fn save(dir: &Path, simulated: bool, p: &Preferences) -> AppResult<Preferences> {
    let p = p.clone().normalize()?;
    fs::create_dir_all(dir).map_err(|e| AppError::Config(e.to_string()))?;
    let text = toml::to_string_pretty(&p).map_err(|e| AppError::Config(e.to_string()))?;
    let target = path(dir, simulated);
    let tmp = target.with_extension("toml.tmp");
    fs::write(&tmp, text).map_err(|e| AppError::Config(e.to_string()))?;
    fs::rename(&tmp, &target).map_err(|e| AppError::Config(e.to_string()))?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let dir = tempfile::tempdir().unwrap();
        let p = load(dir.path(), false);
        assert!(p.notifications.enabled && p.notifications.weekly_summary && p.minimize_to_tray);
        assert_eq!(p.notifications.end_of_day_time, "16:30");
        assert!(!p.start_with_windows);
        assert!(p.extra_holidays.is_empty());
        let j = serde_json::to_value(&p).unwrap();
        assert_eq!(j["notifications"]["endOfDayTime"], "16:30");
        assert_eq!(j["startWithWindows"], false);
        assert_eq!(j["minimizeToTray"], true);
        assert!(j["extraHolidays"].is_object());
    }

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = Preferences::default();
        p.notifications.end_of_day_time = "17:05".into();
        p.notifications.enabled = false;
        p.start_with_windows = true;
        p.extra_holidays.insert("2026-12-24".into(), true);
        p.extra_holidays.insert("2026-11-03".into(), false);
        let saved = save(dir.path(), false, &p).unwrap();
        assert_eq!(saved, p);
        assert_eq!(load(dir.path(), false), p);
        // The simulated mode uses another file.
        assert_eq!(load(dir.path(), true), Preferences::default());
        assert!(path(dir.path(), true).to_string_lossy().ends_with("preferences.simulated.toml"));
    }

    #[test]
    fn corrupt_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(path(dir.path(), false), "this is not toml [[[").unwrap();
        assert_eq!(load(dir.path(), false), Preferences::default());
    }

    #[test]
    fn partial_file_fills_in_defaults() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(path(dir.path(), false), "startWithWindows = true\n").unwrap();
        let p = load(dir.path(), false);
        assert!(p.start_with_windows);
        assert_eq!(p.notifications, NotificationPrefs::default());
    }

    #[test]
    fn invalid_saved_time_is_reset() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(path(dir.path(), false), "startWithWindows = true\n[notifications]\nendOfDayTime = \"25:99\"\n")
            .unwrap();
        let p = load(dir.path(), false);
        assert_eq!(p.notifications.end_of_day_time, "16:30");
        assert!(p.start_with_windows);
    }

    #[test]
    fn invalid_time_on_save_is_a_format_error() {
        let dir = tempfile::tempdir().unwrap();
        for bad in [
            "24:00", "12:60", "9:30", "1230", "ab:cd", "", "12:3", "12:30:00", "+1:30", ":", "12:", ":30",
            "１２:３０", "é1:30", "12:3é", "12 :30",
        ] {
            let mut p = Preferences::default();
            p.notifications.end_of_day_time = bad.into();
            let e = save(dir.path(), false, &p).unwrap_err();
            assert_eq!(e.code(), "format", "{bad:?}");
        }
        assert!(!path(dir.path(), false).exists());
        // Spaces at the ends are trimmed before validating.
        for good in ["00:00", "23:59", "08:05", " 12:30 "] {
            let mut p = Preferences::default();
            p.notifications.end_of_day_time = good.into();
            assert!(p.normalize().is_ok(), "{good}");
        }
    }

    #[test]
    fn invalid_holiday_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = Preferences::default();
        p.extra_holidays.insert("2026-02-30".into(), true);
        p.extra_holidays.insert("tomorrow".into(), true);
        p.extra_holidays.insert("2026-1-5".into(), true);
        p.extra_holidays.insert("2026-12-24".into(), true);
        let saved = save(dir.path(), false, &p).unwrap();
        assert_eq!(saved.extra_holidays.len(), 1);
        assert_eq!(saved.extra_holidays.get("2026-12-24"), Some(&true));
        assert_eq!(load(dir.path(), false), saved);
    }

    #[test]
    fn accepts_the_ui_json() {
        let j = serde_json::json!({
            "notifications": { "enabled": true, "endOfDayTime": "16:30", "weeklySummary": false },
            "startWithWindows": false, "minimizeToTray": true,
            "extraHolidays": { "2026-12-24": true }
        });
        let p: Preferences = serde_json::from_value(j).unwrap();
        assert!(!p.notifications.weekly_summary);
        assert!(p.extra_holidays["2026-12-24"]);
    }

    #[test]
    fn affects_monitor_equal_is_false() {
        let base = Preferences::default();
        assert!(!base.affects_monitor(&base.clone()));
    }

    #[test]
    fn affects_monitor_end_of_day_time() {
        let base = Preferences::default();
        let mut other = base.clone();
        other.notifications.end_of_day_time = "17:00".into();
        assert!(base.affects_monitor(&other));
    }

    #[test]
    fn affects_monitor_weekly_summary() {
        let base = Preferences::default();
        let mut other = base.clone();
        other.notifications.weekly_summary = false;
        assert!(base.affects_monitor(&other));
    }

    #[test]
    fn affects_monitor_enabled() {
        let base = Preferences::default();
        let mut other = base.clone();
        other.notifications.enabled = false;
        assert!(base.affects_monitor(&other));
    }

    #[test]
    fn affects_monitor_add_holiday() {
        let base = Preferences::default();
        let mut other = base.clone();
        other.extra_holidays.insert("2026-12-24".into(), true);
        assert!(base.affects_monitor(&other));
    }

    #[test]
    fn affects_monitor_remove_holiday() {
        let mut base = Preferences::default();
        base.extra_holidays.insert("2026-12-24".into(), true);
        let mut other = base.clone();
        other.extra_holidays.clear();
        assert!(base.affects_monitor(&other));
    }

    #[test]
    fn affects_monitor_only_start_with_windows() {
        let base = Preferences::default();
        let mut other = base.clone();
        other.start_with_windows = true;
        assert!(!base.affects_monitor(&other));
    }

    #[test]
    fn affects_monitor_only_minimize_to_tray() {
        let base = Preferences::default();
        let mut other = base.clone();
        other.minimize_to_tray = false;
        assert!(!base.affects_monitor(&other));
    }
}
