//! Builds the complete `View` (Contract D) from the `ViewInput`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::calendar::{
    add_days, is_holiday, is_monday, iso_week, validate_date, week_holidays, week_target, workdays, ExtraHolidays,
};
use super::hours::{hours_state, WeekState};
use super::issue::{derive, DerivedIssue};
use super::week::{boards, day_secs, last_activity, week_stats, Stats};
use crate::error::{ErrorApp, Resultado};

pub const ALL_BOARDS: &str = "Todos";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewInput {
    pub issues: Vec<Map<String, Value>>,
    pub week: String,
    pub board: String,
    #[serde(default)]
    pub extra_holidays: ExtraHolidays,
    #[serde(default)]
    pub today: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HoursState {
    #[serde(flatten)]
    pub week_state: WeekState,
    pub prev_secs: i64,
    pub workdays: i64,
    pub days: Vec<DayHours>,
    pub holidays: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DayHours {
    pub day: String,
    pub secs: i64,
    pub holiday: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChartWeek {
    pub wk: String,
    pub iso_week: i64,
    pub secs: i64,
    pub per_board: BTreeMap<String, i64>,
    /// The week's target, in hours.
    pub target: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BoardRow {
    pub board: String,
    pub secs: i64,
    pub worked: i64,
    pub resolved: i64,
    pub open: i64,
    pub statuses: BTreeMap<String, i64>,
    pub last: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub today: String,
    pub week: String,
    pub board: String,
    pub issues: Vec<DerivedIssue>,
    pub boards: Vec<String>,
    pub stats: Stats,
    pub hours: HoursState,
    pub weeks: Vec<ChartWeek>,
    pub by_board: Vec<BoardRow>,
}

/// Computes the view. `default_today` is used when the input carries no `today`.
pub fn compute_view(input: ViewInput, default_today: &str) -> Resultado<View> {
    let week = validate_date(&input.week)?;
    if !is_monday(&week)? {
        return Err(ErrorApp::Formato(format!("La semana debe empezar en lunes: «{week}»")));
    }
    let today = validate_date(input.today.as_deref().unwrap_or(default_today))?;
    let extra = &input.extra_holidays;

    let issues = derive(&input.issues)?;
    let names = boards(&issues, &week)?;
    let board = if input.board == ALL_BOARDS || names.contains(&input.board) { input.board.clone() } else { ALL_BOARDS.into() };
    let in_board = |t: &DerivedIssue| board == ALL_BOARDS || t.c.board == board;

    let stats = week_stats(&issues, &week, &in_board)?;

    let week_state = hours_state(&issues, &week, &today, extra)?;
    let prev_secs = week_stats(&issues, &add_days(&week, -7)?, &|_| true)?.secs;
    let mut days = Vec::new();
    for k in 0..5 {
        let day = add_days(&week, k)?;
        days.push(DayHours { secs: day_secs(&issues, &day), holiday: is_holiday(&day, extra), day });
    }
    let hours = HoursState {
        week_state,
        prev_secs,
        workdays: workdays(&week, extra)?.len() as i64,
        days,
        holidays: week_holidays(&week, extra)?,
    };

    let mut weeks = Vec::new();
    for k in (0..=11).rev() {
        let wk = add_days(&week, -7 * k)?;
        let s = week_stats(&issues, &wk, &in_board)?;
        weeks.push(ChartWeek { iso_week: iso_week(&wk)?.1, secs: s.secs, per_board: s.per_board, target: week_target(&wk, extra)?, wk });
    }

    let mut by_board = Vec::new();
    for b in &names {
        let s = week_stats(&issues, &week, &|t| t.c.board == *b)?;
        let mut statuses = BTreeMap::new();
        let mut open = 0;
        for t in issues.iter().filter(|t| t.c.board == *b && t.c.cat.as_deref() != Some("done")) {
            open += 1;
            // `st[i.status]` in JS: an absent status becomes the text «null».
            *statuses.entry(t.c.status.clone().unwrap_or_else(|| "null".into())).or_insert(0) += 1;
        }
        by_board.push(BoardRow {
            board: b.clone(),
            secs: s.secs,
            worked: s.worked.len() as i64,
            resolved: s.resolved.len() as i64,
            open,
            statuses,
            last: last_activity(&issues, b)?,
        });
    }

    Ok(View { today, week, board, issues, boards: names, stats, hours, weeks, by_board })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn input(week: &str, board: &str, today: Option<&str>) -> ViewInput {
        let issues = json!([
            { "key": "A-1", "project": "ADN", "status": "En curso", "cat": "indeterminate", "worklogs": [{ "d": "2026-11-10T10:00:00.000-05:00", "s": 7200 }] },
            { "key": "A-2", "project": "ADN", "status": null, "cat": "new", "worklogs": [] },
            { "key": "G-1", "project": "GPS", "type": "Tarea", "status": "Hecho", "cat": "done", "finDev": "2026-11-11", "worklogs": [] }
        ]);
        ViewInput {
            issues: issues.as_array().unwrap().iter().map(|x| x.as_object().unwrap().clone()).collect(),
            week: week.into(),
            board: board.into(),
            extra_holidays: ExtraHolidays::new(),
            today: today.map(String::from),
        }
    }

    #[test]
    fn full_view() {
        let v = compute_view(input("2026-11-09", "Todos", Some("2026-11-13")), "2000-01-01").unwrap();
        assert_eq!(v.today, "2026-11-13");
        assert_eq!(v.board, "Todos");
        assert_eq!(v.boards, ["Tablero ADN", "Tablero Kanban"]);
        assert_eq!(v.stats.secs, 7200);
        assert_eq!(v.weeks.len(), 12);
        assert_eq!(v.weeks[11].wk, "2026-11-09");
        assert_eq!(v.weeks[0].wk, "2026-08-24");
        assert_eq!(v.weeks[11].iso_week, 46);
        assert_eq!(v.weeks[11].target, 40);
        assert_eq!(v.hours.days.len(), 5);
        assert_eq!(v.hours.days[1].secs, 7200);
        assert_eq!(v.hours.prev_secs, 0);
        assert_eq!(v.hours.workdays, 5);
        assert_eq!(v.by_board.len(), 2);
        assert_eq!(v.by_board[0].board, "Tablero ADN");
        assert_eq!(v.by_board[0].open, 2);
        assert_eq!(v.by_board[0].statuses["En curso"], 1);
        assert_eq!(v.by_board[0].statuses["null"], 1);
        let j = serde_json::to_value(&v).unwrap();
        assert_eq!(j["hours"]["targetH"], 40);
        assert_eq!(j["hours"]["prevSecs"], 0);
        assert_eq!(j["weeks"][0]["isoWeek"], 35);
        assert_eq!(j["issues"][0]["board"], "Tablero ADN");
        assert_eq!(j["byBoard"][0]["open"], 2);
    }

    #[test]
    fn board_filters_and_a_missing_one_goes_back_to_all() {
        let v = compute_view(input("2026-11-09", "Tablero Kanban", Some("2026-11-13")), "").unwrap();
        assert_eq!(v.board, "Tablero Kanban");
        assert_eq!(v.stats.secs, 0);
        assert_eq!(v.stats.resolved, ["G-1"]);
        // `hours` and `byBoard` do not filter.
        assert_eq!(v.hours.week_state.total, 7200);
        assert_eq!(v.by_board.len(), 2);
        let v = compute_view(input("2026-11-09", "No existe", Some("2026-11-13")), "").unwrap();
        assert_eq!(v.board, "Todos");
        assert_eq!(v.stats.secs, 7200);
    }

    #[test]
    fn invalid_week_is_a_format_error() {
        let e = compute_view(input("2026-11-10", "Todos", None), "2026-11-13").unwrap_err();
        assert_eq!(e.codigo(), "formato");
        let e = compute_view(input("hoy", "Todos", None), "2026-11-13").unwrap_err();
        assert_eq!(e.codigo(), "formato");
        let e = compute_view(input("2026-11-09", "Todos", Some("mañana")), "2026-11-13").unwrap_err();
        assert_eq!(e.codigo(), "formato");
    }

    #[test]
    fn default_today_and_issue_order() {
        let v = compute_view(input("2026-11-09", "Todos", None), "2026-11-12").unwrap();
        assert_eq!(v.today, "2026-11-12");
        let keys: Vec<_> = v.issues.iter().map(|t| t.c.key.as_str()).collect();
        assert_eq!(keys, ["A-1", "A-2", "G-1"]);
    }
}
