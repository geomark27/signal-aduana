//! Windows startup according to the `startWithWindows` preference. In simulated mode it never
//! touches the system: it only leaves a log line.

use tauri::{AppHandle, Emitter};
use tauri_plugin_autostart::ManagerExt;

use crate::preferences::{self, Preferences};
use crate::session::AppState;

/// Event that tells the interface the preferences changed internally.
pub const PREFERENCES_EVENT: &str = "signal://preferences";

/// What to do with the autostart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    LogOnly,
    Enable,
    Disable,
    Nothing,
}

/// Simulated → `LogOnly`; wants and not active → `Enable`; does not want and active → `Disable`; otherwise `Nothing`.
pub fn decide(simulated: bool, wants: bool, active: bool) -> Action {
    match (simulated, wants, active) {
        (true, _, _) => Action::LogOnly,
        (false, true, false) => Action::Enable,
        (false, false, true) => Action::Disable,
        _ => Action::Nothing,
    }
}

/// Applies the preference with the autostart plugin (in simulated mode it does not call it).
pub fn sync(app: &AppHandle, simulated: bool, wants: bool) -> Result<(), String> {
    if simulated {
        eprintln!("autostart: simulated mode, Windows startup is not registered (preference: {wants})");
        return Ok(());
    }
    let manager = app.autolaunch();
    let active = manager.is_enabled().map_err(|e| e.to_string())?;
    match decide(simulated, wants, active) {
        Action::Enable => manager.enable().map_err(|e| e.to_string()),
        Action::Disable => manager.disable().map_err(|e| e.to_string()),
        Action::LogOnly | Action::Nothing => Ok(()),
    }
}

/// If the sync failed, the preference is left as `false`. Returns (preferences, changed).
pub fn reconcile(mut p: Preferences, result: &Result<(), String>) -> (Preferences, bool) {
    if result.is_err() {
        p.start_with_windows = false;
        (p, true)
    } else {
        (p, false)
    }
}

/// Syncs and reconciles; if the preference changed, saves it and tells the interface.
pub fn apply(app: &AppHandle, state: &AppState, p: Preferences) -> Preferences {
    let result = sync(app, state.simulated, p.start_with_windows);
    if let Err(e) = &result {
        eprintln!("autostart: could not apply Windows startup: {e}");
    }
    let (p, changed) = reconcile(p, &result);
    if !changed {
        return p;
    }
    let p = match preferences::save(&state.config_dir, state.simulated, &p) {
        Ok(saved) => saved,
        Err(e) => {
            eprintln!("autostart: could not save the reconciled preference: {e}");
            p
        }
    };
    if let Err(e) = app.emit(PREFERENCES_EVENT, &p) {
        eprintln!("autostart: could not emit the preferences change: {e}");
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences::NotificationPrefs;

    #[test]
    fn decide_table() {
        for wants in [false, true] {
            for active in [false, true] {
                assert_eq!(decide(true, wants, active), Action::LogOnly);
            }
        }
        assert_eq!(decide(false, true, false), Action::Enable);
        assert_eq!(decide(false, false, true), Action::Disable);
        assert_eq!(decide(false, true, true), Action::Nothing);
        assert_eq!(decide(false, false, false), Action::Nothing);
    }

    #[test]
    fn reconcile_ok_changes_nothing() {
        let p = Preferences { start_with_windows: true, ..Preferences::default() };
        let (q, changed) = reconcile(p.clone(), &Ok(()));
        assert!(!changed);
        assert_eq!(q, p);
    }

    #[test]
    fn reconcile_error_turns_off_only_the_autostart() {
        let p = Preferences {
            start_with_windows: true,
            notifications: NotificationPrefs { end_of_day_time: "17:45".into(), ..NotificationPrefs::default() },
            ..Preferences::default()
        };
        let (q, changed) = reconcile(p.clone(), &Err("failure".into()));
        assert!(changed);
        assert!(!q.start_with_windows);
        assert_eq!(q.notifications.end_of_day_time, "17:45");
        assert_eq!(q.notifications, p.notifications);
        assert_eq!(q.minimize_to_tray, p.minimize_to_tray);
        assert_eq!(q.extra_holidays, p.extra_holidays);
    }
}
