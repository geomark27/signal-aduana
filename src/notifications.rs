//! Decision of Windows notifications: pure logic, no clock or Tauri.
//! The "now" time comes as a parameter (Ecuador time, UTC-5).

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

use crate::domain::calendar::{fh, DAY_TARGET_H};
use crate::error::{AppError, AppResult};
use crate::preferences::Preferences;
use crate::summary::HoursSummary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    EndOfDay,
    WeeklySummary,
    Reconnect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub kind: NotificationKind,
    pub title: String,
    pub body: String,
}

/// Last date (`YYYY-MM-DD`) on which each notification was sent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationLog {
    pub end_of_day: Option<String>,
    pub weekly_summary: Option<String>,
    pub reconnect: Option<String>,
}

pub enum MonitorState<'a> {
    Summary(&'a HoursSummary),
    Unauthorized,
    NoSession,
    Error,
}

fn date_text(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

fn already_sent(date: &Option<String>, today: &str) -> bool {
    date.as_deref() == Some(today)
}

/// Which notifications are due now. With `prefs.notifications.enabled == false` nothing is sent.
pub fn decide(
    state: MonitorState,
    now: NaiveDateTime,
    prefs: &Preferences,
    log: &NotificationLog,
    simulated: bool,
) -> Vec<Notification> {
    let mut notifications = Vec::new();
    if !prefs.notifications.enabled {
        return notifications;
    }
    let today = date_text(now.date());
    let prefix = if simulated { "[Simulado] " } else { "" };

    match state {
        MonitorState::Unauthorized => {
            if !already_sent(&log.reconnect, &today) {
                notifications.push(Notification {
                    kind: NotificationKind::Reconnect,
                    title: format!("{prefix}Vuelve a conectar Signal Aduana"),
                    body: "Tu token de Jira venció o fue revocado. Abre la app y conéctate de nuevo.".into(),
                });
            }
        }
        // A summary from another day (stale data) is not good for deciding.
        MonitorState::Summary(r) if r.today == today => {
            let day_target = DAY_TARGET_H * 3600;
            let end_of_day = NaiveTime::parse_from_str(&prefs.notifications.end_of_day_time, "%H:%M").ok();
            if r.today_is_workday
                && end_of_day.is_some_and(|c| now.time() >= c)
                && r.today_secs < day_target
                && !already_sent(&log.end_of_day, &today)
            {
                notifications.push(Notification {
                    kind: NotificationKind::EndOfDay,
                    title: format!("{prefix}Te faltan {} hoy", fh(day_target - r.today_secs)),
                    body: format!(
                        "Llevas {} de {} h. Registra tus horas en Jira antes de cerrar el día.",
                        fh(r.today_secs).trim_end_matches(" h"),
                        DAY_TARGET_H
                    ),
                });
            }
            let previous_target = r.previous_week_target_h * 3600;
            if r.first_workday.as_deref() == Some(today.as_str())
                && now.time() >= NaiveTime::from_hms_opt(9, 0, 0).expect("valid time")
                && prefs.notifications.weekly_summary
                && r.previous_week_target_h > 0
                && r.previous_week_secs < previous_target
                && !already_sent(&log.weekly_summary, &today)
            {
                notifications.push(Notification {
                    kind: NotificationKind::WeeklySummary,
                    title: format!("{prefix}La semana pasada faltaron {}", fh(previous_target - r.previous_week_secs)),
                    body: format!(
                        "Registraste {} de {}. Si trabajaste esas horas, regístralas en Jira con su fecha real.",
                        fh(r.previous_week_secs),
                        fh(previous_target)
                    ),
                });
            }
        }
        MonitorState::Summary(_) | MonitorState::NoSession | MonitorState::Error => {}
    }
    notifications
}

/// Records the sent notifications in the log (today's date; the Monday one belongs to that week).
pub fn mark_sent(log: &mut NotificationLog, notifications: &[Notification], today: NaiveDate) {
    let date = Some(date_text(today));
    for n in notifications {
        match n.kind {
            NotificationKind::EndOfDay => log.end_of_day = date.clone(),
            NotificationKind::WeeklySummary => log.weekly_summary = date.clone(),
            NotificationKind::Reconnect => log.reconnect = date.clone(),
        }
    }
}

fn path(dir: &Path, simulated: bool) -> PathBuf {
    dir.join(if simulated { "notifications.simulated.json" } else { "notifications.json" })
}

/// No file or corrupt → empty log.
pub fn load_log(dir: &Path, simulated: bool) -> NotificationLog {
    fs::read_to_string(path(dir, simulated)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn save_log(dir: &Path, simulated: bool, log: &NotificationLog) -> AppResult<()> {
    fs::create_dir_all(dir).map_err(|e| AppError::Config(e.to_string()))?;
    let text = serde_json::to_string_pretty(log).map_err(|e| AppError::Config(e.to_string()))?;
    let target = path(dir, simulated);
    let tmp = target.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|e| AppError::Config(e.to_string()))?;
    fs::rename(&tmp, &target).map_err(|e| AppError::Config(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    fn monday_for(today: &str) -> String {
        let d = NaiveDate::parse_from_str(today, "%Y-%m-%d").unwrap();
        let monday = d - chrono::Duration::days(i64::from(chrono::Datelike::weekday(&d).num_days_from_monday()));
        date_text(monday)
    }

    fn summary(today: &str) -> HoursSummary {
        HoursSummary {
            today: today.into(),
            week_secs: 0,
            week_target_h: 40,
            today_secs: 3 * 3600,
            today_is_workday: true,
            previous_week_secs: 32 * 3600,
            previous_week_target_h: 40,
            first_workday: Some(monday_for(today)),
        }
    }

    fn kinds(v: &[Notification]) -> Vec<NotificationKind> {
        v.iter().map(|n| n.kind).collect()
    }

    fn with_summary(r: &HoursSummary, t: &str) -> Vec<Notification> {
        decide(MonitorState::Summary(r), now(t), &Preferences::default(), &NotificationLog::default(), false)
    }

    #[test]
    fn end_of_day_before_and_after_the_time() {
        let r = summary("2026-11-13"); // Friday
        assert!(with_summary(&r, "2026-11-13 16:29").is_empty());
        let v = with_summary(&r, "2026-11-13 16:30");
        assert_eq!(kinds(&v), [NotificationKind::EndOfDay]);
        assert_eq!(v[0].title, "Te faltan 5 h hoy");
        assert_eq!(v[0].body, "Llevas 3 de 8 h. Registra tus horas en Jira antes de cerrar el día.");
        let mut r2 = summary("2026-11-13");
        r2.today_secs = 7 * 3600 + 1800;
        let v = with_summary(&r2, "2026-11-13 18:00");
        assert_eq!(v[0].title, "Te faltan 0.5 h hoy");
        assert!(v[0].body.starts_with("Llevas 7.5 de 8 h."));
    }

    #[test]
    fn end_of_day_respects_configured_time() {
        let r = summary("2026-11-13");
        let mut p = Preferences::default();
        p.notifications.end_of_day_time = "18:00".into();
        let d = |t| decide(MonitorState::Summary(&r), now(t), &p, &NotificationLog::default(), false);
        assert!(d("2026-11-13 17:59").is_empty());
        assert_eq!(kinds(&d("2026-11-13 18:00")), [NotificationKind::EndOfDay]);
    }

    #[test]
    fn end_of_day_not_on_holiday_weekend_or_full_day() {
        let mut r = summary("2026-11-13");
        r.today_is_workday = false; // holiday
        assert!(with_summary(&r, "2026-11-13 17:00").is_empty());
        let mut s = summary("2026-11-14"); // Saturday
        s.today_is_workday = false;
        assert!(with_summary(&s, "2026-11-14 17:00").is_empty());
        let mut c = summary("2026-11-13");
        c.today_secs = 8 * 3600;
        assert!(with_summary(&c, "2026-11-13 17:00").is_empty());
        c.today_secs = 10 * 3600;
        assert!(with_summary(&c, "2026-11-13 17:00").is_empty());
    }

    #[test]
    fn end_of_day_already_sent_today() {
        let r = summary("2026-11-13");
        let mut log = NotificationLog { end_of_day: Some("2026-11-13".into()), ..NotificationLog::default() };
        let p = Preferences::default();
        assert!(decide(MonitorState::Summary(&r), now("2026-11-13 17:00"), &p, &log, false).is_empty());
        // Sent yesterday: today it is sent.
        log.end_of_day = Some("2026-11-12".into());
        assert_eq!(decide(MonitorState::Summary(&r), now("2026-11-13 17:00"), &p, &log, false).len(), 1);
    }

    #[test]
    fn monday_before_and_after_nine() {
        let mut r = summary("2026-11-16");
        r.today_secs = 0;
        assert!(with_summary(&r, "2026-11-16 08:59").is_empty());
        let v = with_summary(&r, "2026-11-16 09:00");
        assert_eq!(kinds(&v), [NotificationKind::WeeklySummary]);
        assert_eq!(v[0].title, "La semana pasada faltaron 8 h");
        assert_eq!(
            v[0].body,
            "Registraste 32 h de 40 h. Si trabajaste esas horas, regístralas en Jira con su fecha real."
        );
        // After the end-of-day time, on Monday both go out.
        assert_eq!(
            kinds(&with_summary(&r, "2026-11-16 17:00")),
            [NotificationKind::EndOfDay, NotificationKind::WeeklySummary]
        );
    }

    #[test]
    fn monday_does_not_apply_if_complete_or_no_target_or_other_day() {
        let mut r = summary("2026-11-16");
        r.previous_week_secs = 40 * 3600;
        assert!(with_summary(&r, "2026-11-16 09:30").is_empty());
        r.previous_week_secs = 45 * 3600;
        assert!(with_summary(&r, "2026-11-16 09:30").is_empty());
        // Previous week all holidays: target 0.
        r.previous_week_target_h = 0;
        r.previous_week_secs = 0;
        assert!(with_summary(&r, "2026-11-16 09:30").is_empty());
        // Tuesday with an incomplete previous week: no.
        let t = summary("2026-11-17");
        assert!(with_summary(&t, "2026-11-17 10:00").is_empty());
    }

    #[test]
    fn monday_already_sent_and_disabled() {
        let r = summary("2026-11-16");
        let log = NotificationLog { weekly_summary: Some("2026-11-16".into()), ..NotificationLog::default() };
        let p = Preferences::default();
        assert!(decide(MonitorState::Summary(&r), now("2026-11-16 10:00"), &p, &log, false).is_empty());
        let mut p = Preferences::default();
        p.notifications.weekly_summary = false;
        assert!(
            decide(MonitorState::Summary(&r), now("2026-11-16 10:00"), &p, &NotificationLog::default(), false)
                .is_empty()
        );
    }

    #[test]
    fn disabled_notifications_send_nothing() {
        let mut p = Preferences::default();
        p.notifications.enabled = false;
        let r = summary("2026-11-16");
        let log = NotificationLog::default();
        assert!(decide(MonitorState::Summary(&r), now("2026-11-16 17:00"), &p, &log, false).is_empty());
        assert!(decide(MonitorState::Unauthorized, now("2026-11-16 17:00"), &p, &log, false).is_empty());
    }

    #[test]
    fn reconnect_once_a_day() {
        let p = Preferences::default();
        let mut log = NotificationLog::default();
        let v = decide(MonitorState::Unauthorized, now("2026-11-13 10:00"), &p, &log, false);
        assert_eq!(kinds(&v), [NotificationKind::Reconnect]);
        assert_eq!(v[0].title, "Vuelve a conectar Signal Aduana");
        assert_eq!(v[0].body, "Tu token de Jira venció o fue revocado. Abre la app y conéctate de nuevo.");
        mark_sent(&mut log, &v, now("2026-11-13 10:00").date());
        assert!(decide(MonitorState::Unauthorized, now("2026-11-13 15:00"), &p, &log, false).is_empty());
        assert_eq!(decide(MonitorState::Unauthorized, now("2026-11-14 10:00"), &p, &log, false).len(), 1);
    }

    #[test]
    fn no_session_and_error_do_not_notify() {
        let p = Preferences::default();
        let log = NotificationLog::default();
        assert!(decide(MonitorState::NoSession, now("2026-11-16 17:00"), &p, &log, false).is_empty());
        assert!(decide(MonitorState::Error, now("2026-11-16 17:00"), &p, &log, false).is_empty());
    }

    #[test]
    fn summary_from_another_day_does_not_decide() {
        let r = summary("2026-11-12");
        assert!(with_summary(&r, "2026-11-13 17:00").is_empty());
    }

    fn with_first_workday(today: &str, first: Option<&str>) -> HoursSummary {
        let mut r = summary(today);
        r.first_workday = first.map(String::from);
        r
    }

    #[test]
    fn invalid_end_of_day_time_does_not_notify_or_panic() {
        let mut p = Preferences::default();
        p.notifications.end_of_day_time = "99:99".into();
        let log = NotificationLog::default();
        let friday = summary("2026-11-13");
        assert!(decide(MonitorState::Summary(&friday), now("2026-11-13 17:00"), &p, &log, false).is_empty());
        let monday = with_first_workday("2026-11-16", Some("2026-11-16"));
        let v = decide(MonitorState::Summary(&monday), now("2026-11-16 10:00"), &p, &log, false);
        assert_eq!(kinds(&v), [NotificationKind::WeeklySummary]);
    }

    #[test]
    fn summary_sent_last_week_notifies_again() {
        let r = with_first_workday("2026-11-16", Some("2026-11-16"));
        let mut log = NotificationLog { weekly_summary: Some("2026-11-09".into()), ..NotificationLog::default() };
        let p = Preferences::default();
        let t = now("2026-11-16 10:00");
        let v = decide(MonitorState::Summary(&r), t, &p, &log, false);
        assert_eq!(kinds(&v), [NotificationKind::WeeklySummary]);
        mark_sent(&mut log, &v, t.date());
        assert_eq!(log.weekly_summary.as_deref(), Some("2026-11-16"));
    }

    #[test]
    fn holiday_monday_sends_no_summary() {
        let mut r = with_first_workday("2026-11-16", Some("2026-11-17"));
        r.today_is_workday = false;
        assert!(with_summary(&r, "2026-11-16 10:00").is_empty());
    }

    #[test]
    fn summary_goes_out_tuesday_if_monday_was_holiday() {
        let r = with_first_workday("2026-11-17", Some("2026-11-17"));
        assert_eq!(kinds(&with_summary(&r, "2026-11-17 10:00")), [NotificationKind::WeeklySummary]);
        assert!(with_summary(&r, "2026-11-17 08:59").is_empty());
    }

    #[test]
    fn normal_tuesday_sends_no_summary() {
        let r = with_first_workday("2026-11-17", Some("2026-11-16"));
        assert!(!kinds(&with_summary(&r, "2026-11-17 10:00")).contains(&NotificationKind::WeeklySummary));
    }

    #[test]
    fn all_holiday_week_sends_no_summary() {
        let r = with_first_workday("2026-11-16", None);
        assert!(!kinds(&with_summary(&r, "2026-11-16 10:00")).contains(&NotificationKind::WeeklySummary));
    }

    #[test]
    fn simulated_has_prefix() {
        let r = summary("2026-11-16");
        let p = Preferences::default();
        let log = NotificationLog::default();
        let v = decide(MonitorState::Summary(&r), now("2026-11-16 17:00"), &p, &log, true);
        assert_eq!(v.len(), 2);
        assert!(v.iter().all(|n| n.title.starts_with("[Simulado] ")));
        let v = decide(MonitorState::Unauthorized, now("2026-11-16 17:00"), &p, &log, true);
        assert_eq!(v[0].title, "[Simulado] Vuelve a conectar Signal Aduana");
    }

    #[test]
    fn mark_sent_and_log_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_log(dir.path(), false), NotificationLog::default());
        let r = summary("2026-11-16");
        let today = now("2026-11-16 17:00");
        let mut log = NotificationLog::default();
        let v = decide(MonitorState::Summary(&r), today, &Preferences::default(), &log, false);
        assert_eq!(v.len(), 2);
        mark_sent(&mut log, &v, today.date());
        assert_eq!(log.end_of_day.as_deref(), Some("2026-11-16"));
        assert_eq!(log.weekly_summary.as_deref(), Some("2026-11-16"));
        assert_eq!(log.reconnect, None);
        save_log(dir.path(), false, &log).unwrap();
        let loaded = load_log(dir.path(), false);
        assert_eq!(loaded, log);
        assert!(decide(MonitorState::Summary(&r), today, &Preferences::default(), &loaded, false).is_empty());
        // The simulated mode uses another file; a corrupt file gives an empty log.
        assert_eq!(load_log(dir.path(), true), NotificationLog::default());
        fs::write(path(dir.path(), false), "{ broken").unwrap();
        assert_eq!(load_log(dir.path(), false), NotificationLog::default());
    }
}
