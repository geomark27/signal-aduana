//! Per-week statistics, board ordering and last activity
//! (`weekStats`, `boards`, `lastActivity`, `daySecs` from `ui/app.js`).

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::Serialize;

use super::calendar::{add_days, date_only};
use super::issue::DerivedIssue;
use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub secs: i64,
    /// Keys of issues with hours in the week, in the order of `issues`.
    pub worked: Vec<String>,
    pub per_board: BTreeMap<String, i64>,
    pub per_day: BTreeMap<String, BTreeMap<String, i64>>,
    /// Keys of issues concluded in the week, in the order of `issues`.
    pub resolved: Vec<String>,
    pub est: i64,
    pub spent: i64,
    pub with_due: i64,
    pub on_time: i64,
}

/// `weekStats(wk, filter)`. Weekend worklogs (null `wk`) do not add up.
pub fn week_stats(issues: &[DerivedIssue], wk: &str, filter: &dyn Fn(&DerivedIssue) -> bool) -> AppResult<Stats> {
    let end = add_days(wk, 4)?;
    let mut st = Stats {
        secs: 0,
        worked: Vec::new(),
        per_board: BTreeMap::new(),
        per_day: BTreeMap::new(),
        resolved: Vec::new(),
        est: 0,
        spent: 0,
        with_due: 0,
        on_time: 0,
    };
    for t in issues.iter().filter(|t| filter(t)) {
        let mut has_hours = false;
        for w in t.c.worklogs.iter().filter(|w| w.wk.as_deref() == Some(wk)) {
            st.secs += w.s;
            has_hours = true;
            *st.per_board.entry(t.c.board.clone()).or_insert(0) += w.s;
            *st.per_day.entry(w.day.clone()).or_default().entry(t.c.board.clone()).or_insert(0) += w.s;
        }
        if has_hours {
            st.worked.push(t.c.key.clone());
        }
        if let Some(r) = t.c.res_day.as_deref() {
            if r >= wk && r <= end.as_str() {
                st.resolved.push(t.c.key.clone());
                st.est += t.c.est;
                st.spent += t.c.spent;
                if let Some(due) = t.c.due.as_deref() {
                    st.with_due += 1;
                    if r <= due {
                        st.on_time += 1;
                    }
                }
            }
        }
    }
    Ok(st)
}

/// `daySecs(day)`: all the hours of a day, weekends included.
pub fn day_secs(issues: &[DerivedIssue], day: &str) -> i64 {
    issues.iter().flat_map(|t| &t.c.worklogs).filter(|w| w.day == day).map(|w| w.s).sum()
}

/// `lastActivity(b)`: the most recent date among hours, resolution and update (`""` if none).
pub fn last_activity(issues: &[DerivedIssue], board: &str) -> AppResult<String> {
    let mut last = String::new();
    for t in issues.iter().filter(|t| t.c.board == board) {
        for w in &t.c.worklogs {
            if w.day > last {
                last = w.day.clone();
            }
        }
        if let Some(r) = &t.c.res_day {
            if *r > last {
                last = r.clone();
            }
        }
        if let Some(u) = date_only(t.c.updated.as_deref())? {
            if u > last {
                last = u;
            }
        }
    }
    Ok(last)
}

/// `boards()`: unique names ordered by the week's hours, concluded in the week, open and
/// last activity (all from highest to lowest) and, finally, by name.
pub fn boards(issues: &[DerivedIssue], wk: &str) -> AppResult<Vec<String>> {
    let mut names: Vec<String> = Vec::new();
    for t in issues {
        if !names.contains(&t.c.board) {
            names.push(t.c.board.clone());
        }
    }
    let mut scores: BTreeMap<String, (i64, i64, i64, String)> = BTreeMap::new();
    for b in &names {
        let s = week_stats(issues, wk, &|t| t.c.board == *b)?;
        let open = issues.iter().filter(|t| t.c.board == *b && t.c.cat.as_deref() != Some("done")).count() as i64;
        scores.insert(b.clone(), (s.secs, s.resolved.len() as i64, open, last_activity(issues, b)?));
    }
    names.sort_by(|a, b| {
        let (x, y) = (&scores[a], &scores[b]);
        y.0.cmp(&x.0)
            .then(y.1.cmp(&x.1))
            .then(y.2.cmp(&x.2))
            .then_with(|| compare_locale(&y.3, &x.3))
            .then_with(|| compare_locale(a, b))
    });
    Ok(names)
}

const PUNCTUATION: &str = "_-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$";

/// Primary key of a character according to ICU order (what `localeCompare` uses):
/// spaces, punctuation, digits and letters (ignoring case and accents).
fn primary_key(c: char) -> (u8, u32) {
    if c.is_whitespace() {
        return (0, 0);
    }
    if let Some(p) = PUNCTUATION.find(c) {
        return (1, p as u32);
    }
    if c.is_ascii_digit() {
        return (2, c as u32);
    }
    let base = match c.to_lowercase().next().unwrap_or(c) {
        'á' | 'à' | 'ä' | 'â' | 'ã' => 'a',
        'é' | 'è' | 'ë' | 'ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 'o',
        'ú' | 'ù' | 'ü' | 'û' => 'u',
        'ñ' => 'n',
        other => other,
    };
    (3, base as u32)
}

/// Approximation of `String.prototype.localeCompare`: letters ignoring case/accents
/// first; on a tie, accents; and last, lowercase before uppercase.
pub fn compare_locale(a: &str, b: &str) -> Ordering {
    let pa: Vec<_> = a.chars().map(primary_key).collect();
    let pb: Vec<_> = b.chars().map(primary_key).collect();
    pa.cmp(&pb)
        .then_with(|| {
            let accent = |s: &str| s.chars().map(|c| !c.is_ascii() && c.is_alphabetic() && primary_key(c).0 == 3).collect::<Vec<_>>();
            accent(a).cmp(&accent(b))
        })
        .then_with(|| {
            let upper = |s: &str| s.chars().map(|c| c.is_uppercase()).collect::<Vec<_>>();
            upper(a).cmp(&upper(b))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::issue::derive;
    use serde_json::{json, Value};

    fn issues(v: Value) -> Vec<DerivedIssue> {
        let m: Vec<_> = v.as_array().unwrap().iter().map(|x| x.as_object().unwrap().clone()).collect();
        derive(&m).unwrap()
    }

    fn all(_: &DerivedIssue) -> bool {
        true
    }

    fn data() -> Vec<DerivedIssue> {
        issues(json!([
            { "key": "A-1", "project": "ADN", "cat": "done", "finDev": "2026-11-11", "due": "2026-11-10", "est": 3600, "spent": 7200,
              "worklogs": [
                { "d": "2026-11-09T10:00:00.000-05:00", "s": 3600 },
                { "d": "2026-11-14T10:00:00.000-05:00", "s": 5000 },
                { "d": "2026-11-02T10:00:00.000-05:00", "s": 100 } ] },
            { "key": "A-2", "project": "ADN", "cat": "done", "finDev": "2026-11-13", "due": "2026-11-13", "est": 1800, "spent": 1800, "worklogs": [] },
            { "key": "B-1", "project": "GPS", "type": "Tarea", "cat": "indeterminate", "est": 0, "spent": 0,
              "worklogs": [ { "d": "2026-11-10T10:00:00.000-05:00", "s": 1800 } ] },
            { "key": "B-2", "project": "GPS", "type": "Tarea", "cat": "done", "resolved": "2026-11-15T12:00:00.000-05:00", "est": 0, "spent": 0, "worklogs": [] }
        ]))
    }

    #[test]
    fn week_statistics() {
        let d = data();
        let s = week_stats(&d, "2026-11-09", &all).unwrap();
        assert_eq!(s.secs, 5400); // without Saturday or the previous week
        assert_eq!(s.worked, ["A-1", "B-1"]);
        assert_eq!(s.resolved, ["A-1", "A-2"]); // B-2 closed on Sunday: out
        assert_eq!((s.est, s.spent), (5400, 9000));
        assert_eq!((s.with_due, s.on_time), (2, 1));
        assert_eq!(s.per_board["Tablero ADN"], 3600);
        assert_eq!(s.per_board["Tablero Kanban"], 1800);
        assert_eq!(s.per_day["2026-11-09"]["Tablero ADN"], 3600);
        assert!(!s.per_day.contains_key("2026-11-14"));
        // With a board filter.
        let s = week_stats(&d, "2026-11-09", &|t| t.c.board == "Tablero Kanban").unwrap();
        assert_eq!((s.secs, s.worked.len(), s.resolved.len()), (1800, 1, 0));
        // A week with nothing.
        let s = week_stats(&d, "2026-11-16", &all).unwrap();
        assert_eq!((s.secs, s.worked.len(), s.resolved.len()), (0, 0, 0));
    }

    #[test]
    fn day_secs_includes_weekend() {
        let d = data();
        assert_eq!(day_secs(&d, "2026-11-14"), 5000);
        assert_eq!(day_secs(&d, "2026-11-12"), 0);
    }

    #[test]
    fn last_activity_takes_the_maximum() {
        let d = data();
        assert_eq!(last_activity(&d, "Tablero ADN").unwrap(), "2026-11-14");
        assert_eq!(last_activity(&d, "Tablero Kanban").unwrap(), "2026-11-15");
        assert_eq!(last_activity(&d, "Otro").unwrap(), "");
    }

    #[test]
    fn board_order_depends_on_the_week() {
        let d = data();
        // Week of 09-Nov: ADN has 3600 s, Kanban 1800 s.
        assert_eq!(boards(&d, "2026-11-09").unwrap(), ["Tablero ADN", "Tablero Kanban"]);
        // Week of 16-Nov: nobody has hours or concluded; Kanban has 1 open and ADN 0.
        assert_eq!(boards(&d, "2026-11-16").unwrap(), ["Tablero Kanban", "Tablero ADN"]);
    }

    #[test]
    fn ties_are_broken_by_activity_and_name() {
        let d = issues(json!([
            { "key": "X-1", "comp": "Aforo", "cat": "done", "worklogs": [] },
            { "key": "X-2", "comp": "DAI", "cat": "done", "worklogs": [] },
            { "key": "X-3", "comp": "Regímenes Especiales", "cat": "done", "worklogs": [] }
        ]));
        assert_eq!(boards(&d, "2026-11-09").unwrap(), ["Tablero AFO", "Tablero DAI", "Tablero RE"]);
    }

    #[test]
    fn locale_like_comparison() {
        assert_eq!(compare_locale("a", "B"), Ordering::Less);
        assert_eq!(compare_locale("Tablero AFO", "Tablero DAI"), Ordering::Less);
        assert_eq!(compare_locale("Kanban-GPS", "Kanban GPS"), Ordering::Greater);
        assert_eq!(compare_locale("", "2026-11-01"), Ordering::Less);
        assert_eq!(compare_locale("2026-11-02", "2026-11-01"), Ordering::Greater);
        assert_eq!(compare_locale("a", "A"), Ordering::Less);
        assert_eq!(compare_locale("x", "x"), Ordering::Equal);
    }
}
