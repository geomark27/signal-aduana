//! Hours summary for the tray and notifications. Uses `domain::compute_view`:
//! it does not reimplement hours or holiday rules.

use chrono::{Datelike, Duration, NaiveDate};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::domain::calendar::{fh, workdays, ExtraHolidays};
use crate::domain::view::ALL_BOARDS;
use crate::domain::{compute_view, View, ViewInput};
use crate::error::AppResult;

/// Maximum length of the tooltip text (Windows cuts at around 127 characters).
pub const MAX_TRAY_TEXT: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HoursSummary {
    /// `YYYY-MM-DD`
    pub today: String,
    pub week_secs: i64,
    pub week_target_h: i64,
    pub today_secs: i64,
    pub today_is_workday: bool,
    /// Complete previous week.
    pub previous_week_secs: i64,
    pub previous_week_target_h: i64,
    /// First workday of the current week (`YYYY-MM-DD`); `None` if the whole week is holidays.
    pub first_workday: Option<String>,
}

fn view_for(issues: &[Map<String, Value>], week: NaiveDate, today: NaiveDate, holidays: &ExtraHolidays) -> AppResult<View> {
    compute_view(
        ViewInput {
            issues: issues.to_vec(),
            week: week.format("%Y-%m-%d").to_string(),
            board: ALL_BOARDS.into(),
            extra_holidays: holidays.clone(),
            today: Some(today.format("%Y-%m-%d").to_string()),
        },
        &today.format("%Y-%m-%d").to_string(),
    )
}

pub fn summarize(issues: Vec<Map<String, Value>>, today: NaiveDate, holidays: &ExtraHolidays) -> AppResult<HoursSummary> {
    let monday = today - Duration::days(i64::from(today.weekday().num_days_from_monday()));
    let current = view_for(&issues, monday, today, holidays)?;
    let previous = view_for(&issues, monday - Duration::days(7), today, holidays)?;
    let first_workday = workdays(&monday.format("%Y-%m-%d").to_string(), holidays)?.into_iter().next();
    Ok(HoursSummary {
        today: current.today.clone(),
        week_secs: current.hours.week_state.total,
        week_target_h: current.hours.week_state.target_h,
        today_secs: current.hours.week_state.today_secs,
        today_is_workday: current.hours.week_state.today_labor,
        previous_week_secs: previous.hours.week_state.total,
        previous_week_target_h: previous.hours.week_state.target_h,
        first_workday,
    })
}

/// Tray tooltip text, e.g. `Signal Aduana · Semana: 24 de 40 h · Hoy: 3 de 8 h`.
pub fn tray_text(summary: Option<&HoursSummary>, simulated: bool) -> String {
    let mut text = match summary {
        None => "Signal Aduana · Sin datos".to_string(),
        Some(r) => {
            let week = format!("Semana: {} de {} h", hours_without_unit(r.week_secs), r.week_target_h);
            if r.today_is_workday {
                format!("Signal Aduana · {week} · Hoy: {} de 8 h", hours_without_unit(r.today_secs))
            } else {
                format!("Signal Aduana · {week} · Hoy: no laborable")
            }
        }
    };
    if simulated {
        text.push_str(" (simulado)");
    }
    text.chars().take(MAX_TRAY_TEXT).collect()
}

/// `fh` without the " h" suffix (`7.5 h` → `7.5`).
fn hours_without_unit(s: i64) -> String {
    fh(s).trim_end_matches(" h").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jira::simulated::SimulatedJira;
    use crate::jira::{self, Source};

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    async fn issues(today: NaiveDate) -> Vec<Map<String, Value>> {
        let f = Source::Simulated(SimulatedJira::new("persona@ejemplo.com", true, today));
        let me = f.myself().await.unwrap().account_id;
        jira::load_for_summary(&f, Some(&me), today).await.unwrap()
    }

    async fn against_view(today: &str, extra: &ExtraHolidays) {
        let h = date(today);
        let ts = issues(h).await;
        let r = summarize(ts.clone(), h, extra).unwrap();
        let monday = h - Duration::days(i64::from(h.weekday().num_days_from_monday()));
        let mk = |week: NaiveDate| {
            compute_view(
                ViewInput {
                    issues: ts.clone(),
                    week: week.to_string(),
                    board: "Todos".into(),
                    extra_holidays: extra.clone(),
                    today: Some(today.into()),
                },
                today,
            )
            .unwrap()
        };
        let (a, p) = (mk(monday), mk(monday - Duration::days(7)));
        assert_eq!(r.today, today);
        assert_eq!(r.week_secs, a.hours.week_state.total);
        assert_eq!(r.week_target_h, a.hours.week_state.target_h);
        assert_eq!(r.today_secs, a.hours.week_state.today_secs);
        assert_eq!(r.today_is_workday, a.hours.week_state.today_labor);
        assert_eq!(r.previous_week_secs, p.hours.week_state.total);
        assert_eq!(r.previous_week_target_h, p.weeks[11].target);
        assert_eq!(r.week_target_h, a.weeks[11].target);
    }

    #[tokio::test]
    async fn summarizes_friday_and_monday_like_the_view() {
        let empty = ExtraHolidays::new();
        against_view("2026-11-13", &empty).await;
        against_view("2026-11-16", &empty).await;
        let mut extra = ExtraHolidays::new();
        extra.insert("2026-11-10".into(), true);
        against_view("2026-11-13", &extra).await;
    }

    #[tokio::test]
    async fn concrete_simulated_values() {
        let r = summarize(issues(date("2026-11-13")).await, date("2026-11-13"), &ExtraHolidays::new()).unwrap();
        assert!(r.today_is_workday);
        assert!(r.week_target_h > 0 && r.previous_week_target_h > 0);
        assert!(r.week_secs >= r.today_secs);
        let l = summarize(issues(date("2026-11-16")).await, date("2026-11-16"), &ExtraHolidays::new()).unwrap();
        // On Monday, the previous week is the one that ended on Friday the 13th (full target).
        assert_eq!(l.previous_week_target_h, 40);
        assert_eq!(l.today_secs, 0);
    }

    #[test]
    fn weekend_is_not_a_workday() {
        let r = summarize(Vec::new(), date("2026-11-14"), &ExtraHolidays::new()).unwrap();
        assert!(!r.today_is_workday);
        assert_eq!(r.today_secs, 0);
        assert_eq!(r.week_target_h, 40);
    }

    fn summary_with(week: i64, today: i64) -> HoursSummary {
        HoursSummary {
            today: "2026-11-13".into(),
            week_secs: week,
            week_target_h: 40,
            today_secs: today,
            today_is_workday: true,
            previous_week_secs: 0,
            previous_week_target_h: 40,
            first_workday: Some("2026-11-09".into()),
        }
    }

    #[test]
    fn summarize_marks_base_and_extra_holiday() {
        let empty = ExtraHolidays::new();
        let r = summarize(Vec::new(), date("2026-11-03"), &empty).unwrap();
        assert!(!r.today_is_workday);
        assert_eq!(r.week_target_h, 24);
        let mut removes = ExtraHolidays::new();
        removes.insert("2026-11-03".into(), false);
        let r = summarize(Vec::new(), date("2026-11-03"), &removes).unwrap();
        assert!(r.today_is_workday);
        assert_eq!(r.week_target_h, 32);
        let r = summarize(Vec::new(), date("2026-11-14"), &empty).unwrap();
        assert!(!r.today_is_workday);
    }

    #[test]
    fn summarize_computes_first_workday() {
        let first = |today: &str, extra: &ExtraHolidays| summarize(Vec::new(), date(today), extra).unwrap().first_workday;
        let empty = ExtraHolidays::new();
        assert_eq!(first("2026-11-18", &empty).as_deref(), Some("2026-11-16"));
        let mut monday_holiday = ExtraHolidays::new();
        monday_holiday.insert("2026-11-16".into(), true);
        assert_eq!(first("2026-11-18", &monday_holiday).as_deref(), Some("2026-11-17"));
        // November 2 and 3 are base holidays.
        assert_eq!(first("2026-11-04", &empty).as_deref(), Some("2026-11-04"));
        let all: ExtraHolidays =
            ["2026-11-16", "2026-11-17", "2026-11-18", "2026-11-19", "2026-11-20"].iter().map(|d| (d.to_string(), true)).collect();
        assert_eq!(first("2026-11-18", &all), None);
    }

    #[test]
    fn tray_text_formats() {
        let r = summary_with(24 * 3600, 3 * 3600);
        assert_eq!(tray_text(Some(&r), false), "Signal Aduana · Semana: 24 de 40 h · Hoy: 3 de 8 h");
        assert_eq!(tray_text(Some(&r), true), "Signal Aduana · Semana: 24 de 40 h · Hoy: 3 de 8 h (simulado)");
        assert_eq!(tray_text(None, false), "Signal Aduana · Sin datos");
        assert_eq!(tray_text(None, true), "Signal Aduana · Sin datos (simulado)");
        let decimals = summary_with(27000, 5400);
        assert_eq!(tray_text(Some(&decimals), false), "Signal Aduana · Semana: 7.5 de 40 h · Hoy: 1.5 de 8 h");
        let mut holiday = summary_with(0, 0);
        holiday.today_is_workday = false;
        assert!(tray_text(Some(&holiday), false).ends_with("Hoy: no laborable"));
        let huge = summary_with(i64::MAX / 4, i64::MAX / 4);
        assert!(tray_text(Some(&huge), true).chars().count() <= MAX_TRAY_TEXT);
    }
}
