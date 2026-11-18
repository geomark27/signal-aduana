//! State of a week's hours (`hoursState` from `ui/app.js`).
//! The texts come out character by character as in the JS.

use serde::Serialize;

use super::calendar::{add_days, fh, is_holiday, week_target, workdays, ExtraHolidays, DAY_TARGET_H};
use super::issue::DerivedIssue;
use super::week::{day_secs, week_stats};
use crate::error::AppResult;

/// The fields that `hoursState(wk)` returns (without the view's extras).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WeekState {
    pub total: i64,
    pub target: i64,
    pub target_h: i64,
    pub expected: i64,
    pub missing: i64,
    pub behind: i64,
    pub sev: String,
    pub title: String,
    pub detail: String,
    pub current: bool,
    pub today_secs: i64,
    pub today_labor: bool,
    pub today: String,
}

fn days_word(n: usize) -> &'static str {
    if n == 1 { "día" } else { "días" }
}

pub fn hours_state(issues: &[DerivedIssue], wk: &str, today: &str, extra: &ExtraHolidays) -> AppResult<WeekState> {
    let total = week_stats(issues, wk, &|_| true)?.secs;
    let target_h = week_target(wk, extra)?;
    let target = target_h * 3600;
    let end = add_days(wk, 4)?;
    let current = today >= wk && today <= end.as_str();
    let future = today < wk;
    let worked_days = workdays(wk, extra)?.iter().filter(|d| d.as_str() < today).count();
    let today_secs = if current { day_secs(issues, today) } else { 0 };
    let today_labor = current && !is_holiday(today, extra) && today <= end.as_str();
    let day_target_secs = DAY_TARGET_H * 3600;
    let expected = if current { worked_days as i64 * day_target_secs } else { target };
    let missing = (target - total).max(0);
    let behind = (expected - total).max(0);

    let (sev, title, detail): (&str, String, String);
    if target_h == 0 {
        sev = "";
        title = "Semana de feriados".into();
        detail = format!("No hay días laborables en esta semana.{}", if total != 0 { format!(" Registraste {}.", fh(total)) } else { String::new() });
    } else if future {
        sev = "";
        title = "Semana futura".into();
        detail = "Todavía no hay horas para esta semana.".into();
    } else if total >= target {
        sev = "ok";
        title = if total > target { format!("Semana completa: {} registradas (+{})", fh(total), fh(total - target)) } else { "Semana completa".into() };
        detail = if current { "Ya cumpliste las horas de la semana.".into() } else { "Cumpliste las horas de esa semana.".into() };
    } else if !current {
        sev = "crit";
        title = format!("Faltaron {} para completar las {} h", fh(missing), target_h);
        detail = "Esa semana ya cerró. Si trabajaste esas horas, regístralas en Jira con su fecha real.".into();
    } else if behind >= day_target_secs {
        sev = "crit";
        title = format!("Te faltan {} para las {} h", fh(missing), target_h);
        detail = format!(
            "Vas {} por debajo de lo esperado hasta ayer ({} en {} {} laborables). Revisa si te falta registrar horas.",
            fh(behind), fh(expected), worked_days, days_word(worked_days)
        );
    } else if behind > 0 {
        sev = "warn";
        title = format!("Te faltan {} para las {} h", fh(missing), target_h);
        detail = format!("Vas {} por debajo de lo esperado hasta ayer ({}).", fh(behind), fh(expected));
    } else {
        title = format!("Te faltan {} para las {} h", fh(missing), target_h);
        if worked_days > 0 {
            sev = "warn";
            detail = format!("Vas al día: lo esperado hasta ayer era {}.", fh(expected));
        } else {
            sev = "";
            detail = "Empieza la semana: registra tus horas cada día.".into();
        }
    }
    Ok(WeekState {
        total, target, target_h, expected, missing, behind,
        sev: sev.into(), title, detail, current, today_secs, today_labor, today: today.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::issue::derive;
    use serde_json::{json, Value};

    /// Issues with one worklog on each given day, of the given seconds each.
    fn with_hours(days: &[(&str, i64)]) -> Vec<DerivedIssue> {
        let wls: Vec<Value> = days.iter().map(|(d, s)| json!({ "d": format!("{d}T12:00:00.000-05:00"), "s": s })).collect();
        let t = json!({ "key": "A-1", "worklogs": wls });
        derive(&[t.as_object().unwrap().clone()]).unwrap()
    }

    fn state(t: &[DerivedIssue], wk: &str, today: &str) -> WeekState {
        hours_state(t, wk, today, &ExtraHolidays::new()).unwrap()
    }

    const H8: i64 = 28800;

    #[test]
    fn future_week() {
        let e = state(&[], "2026-11-16", "2026-11-13");
        assert_eq!((e.sev.as_str(), e.title.as_str()), ("", "Semana futura"));
        assert_eq!(e.detail, "Todavía no hay horas para esta semana.");
        assert!(!e.current);
        assert_eq!(e.expected, 40 * 3600);
    }

    #[test]
    fn past_week_complete_with_excess_and_incomplete() {
        let c = with_hours(&[("2026-11-09", H8), ("2026-11-10", H8), ("2026-11-11", H8), ("2026-11-12", H8), ("2026-11-13", H8)]);
        let e = state(&c, "2026-11-09", "2026-11-18");
        assert_eq!((e.sev.as_str(), e.title.as_str()), ("ok", "Semana completa"));
        assert_eq!(e.detail, "Cumpliste las horas de esa semana.");

        let c = with_hours(&[("2026-11-09", 3600 * 41)]);
        let e = state(&c, "2026-11-09", "2026-11-18");
        assert_eq!(e.title, "Semana completa: 41 h registradas (+1 h)");

        let c = with_hours(&[("2026-11-09", 27000)]);
        let e = state(&c, "2026-11-09", "2026-11-18");
        assert_eq!(e.sev, "crit");
        assert_eq!(e.title, "Faltaron 32.5 h para completar las 40 h");
        assert!(e.detail.starts_with("Esa semana ya cerró."));
        assert_eq!(e.missing, 40 * 3600 - 27000);
    }

    #[test]
    fn current_week_behind() {
        // Friday: 4 workdays elapsed, expected 32 h; there are 8 h.
        let c = with_hours(&[("2026-11-09", H8)]);
        let e = state(&c, "2026-11-09", "2026-11-13");
        assert!(e.current);
        assert_eq!(e.expected, 4 * H8);
        assert_eq!(e.behind, 3 * H8);
        assert_eq!(e.sev, "crit");
        assert_eq!(e.title, "Te faltan 32 h para las 40 h");
        assert_eq!(e.detail, "Vas 24 h por debajo de lo esperado hasta ayer (32 h en 4 días laborables). Revisa si te falta registrar horas.");
    }

    #[test]
    fn current_week_slightly_behind_and_on_track() {
        // Thursday: expected 24 h (3 days). With 20 h: behind by less than a day.
        let c = with_hours(&[("2026-11-09", 3600 * 20)]);
        let e = state(&c, "2026-11-09", "2026-11-12");
        assert_eq!(e.sev, "warn");
        assert_eq!(e.behind, 4 * 3600);
        assert_eq!(e.detail, "Vas 4 h por debajo de lo esperado hasta ayer (24 h).");

        let c = with_hours(&[("2026-11-09", 3600 * 24)]);
        let e = state(&c, "2026-11-09", "2026-11-12");
        assert_eq!((e.sev.as_str(), e.behind), ("warn", 0));
        assert_eq!(e.title, "Te faltan 16 h para las 40 h");
        assert_eq!(e.detail, "Vas al día: lo esperado hasta ayer era 24 h.");
    }

    #[test]
    fn first_day_of_the_week() {
        let e = state(&[], "2026-11-09", "2026-11-09");
        assert_eq!(e.sev, "");
        assert_eq!(e.detail, "Empieza la semana: registra tus horas cada día.");
        assert_eq!(e.title, "Te faltan 40 h para las 40 h");
        assert!(e.today_labor);
        assert_eq!(e.expected, 0);
    }

    #[test]
    fn holiday_week_and_today() {
        let mut extra = ExtraHolidays::new();
        for d in ["2026-11-09", "2026-11-10", "2026-11-11", "2026-11-12", "2026-11-13"] {
            extra.insert(d.into(), true);
        }
        let c = with_hours(&[("2026-11-09", 5400)]);
        let e = hours_state(&c, "2026-11-09", "2026-11-10", &extra).unwrap();
        assert_eq!(e.title, "Semana de feriados");
        assert_eq!(e.detail, "No hay días laborables en esta semana. Registraste 1.5 h.");
        assert!(!e.today_labor);
        let e = hours_state(&[], "2026-11-09", "2026-11-10", &extra).unwrap();
        assert_eq!(e.detail, "No hay días laborables en esta semana.");
        // A single workday: the singular.
        let mut extra = ExtraHolidays::new();
        for d in ["2026-11-10", "2026-11-11", "2026-11-12", "2026-11-13"] {
            extra.insert(d.into(), true);
        }
        let e = hours_state(&[], "2026-11-09", "2026-11-12", &extra).unwrap();
        assert_eq!(e.sev, "crit");
        assert!(e.detail.contains("(8 h en 1 día laborables)"), "{}", e.detail);
    }

    #[test]
    fn today_hours_count_only_in_the_current_week() {
        let c = with_hours(&[("2026-11-12", 5400)]);
        assert_eq!(state(&c, "2026-11-09", "2026-11-12").today_secs, 5400);
        assert_eq!(state(&c, "2026-11-02", "2026-11-12").today_secs, 0);
        // Saturday of the current week: not `current` (today > Friday).
        assert!(!state(&c, "2026-11-09", "2026-11-14").current);
    }
}
