//! Derived fields of each issue (`prep()` from `ui/app.js`).
//!
//! The input may carry fields Rust does not know (`hasPR`, `docChecked`…): they are kept as
//! they are and returned along with the derived ones. Fields are read from the JSON with the same
//! tolerance as the JS: absent or of another type counts as "no value".

use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;
use serde::Serialize;
use serde_json::{Map, Value};

use super::board::{resolve_board, BoardData, SprintInfo};
use super::calendar::{date_only, instant_ms, js_round, local_date, monday_of};
use crate::error::{AppError, AppResult};

const DAY_MS: f64 = 864e5;

/// `NO_DOC_RE`: issues that do not carry a deployment document.
fn no_doc_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)reuni[oó]n|capacitaci[oó]n|revisi[oó]n de tarea|soporte|an[aá]lisis").unwrap())
}

/// Worklog with its derived fields.
#[derive(Debug, Clone)]
pub struct WorklogCalc {
    pub s: i64,
    pub day: String,
    pub weekend: bool,
    pub wk: Option<String>,
}

/// What the week, hours and board calculations need from an already derived issue.
#[derive(Debug, Clone)]
pub struct Calc {
    pub key: String,
    pub summary: String,
    pub status: Option<String>,
    pub cat: Option<String>,
    pub est: i64,
    pub spent: i64,
    pub due: Option<String>,
    pub updated: Option<String>,
    pub board: String,
    pub board_how: String,
    pub fin_day: Option<String>,
    pub res_day: Option<String>,
    pub no_fin: bool,
    pub late: Option<i64>,
    pub has_wl: bool,
    pub need_doc: Value,
    pub cre_day: Option<String>,
    pub worklogs: Vec<WorklogCalc>,
}

/// An input issue plus the derived fields of `prep()`.
#[derive(Debug, Clone, Serialize)]
pub struct DerivedIssue {
    /// All the received fields (unknown ones too) plus the derived ones.
    #[serde(flatten)]
    pub fields: Map<String, Value>,
    #[serde(skip)]
    pub c: Calc,
}

fn text<'a>(m: &'a Map<String, Value>, k: &str) -> Option<&'a str> {
    m.get(k).and_then(Value::as_str)
}

/// Non-empty text (in JS, the empty string is "falsy").
fn non_empty_text<'a>(m: &'a Map<String, Value>, k: &str) -> Option<&'a str> {
    text(m, k).filter(|s| !s.is_empty())
}

fn integer(m: &Map<String, Value>, k: &str) -> i64 {
    m.get(k).and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))).unwrap_or(0)
}

fn visible(v: Option<&Value>) -> String {
    match v {
        None => "undefined".into(),
        Some(Value::Null) => "null".into(),
        Some(Value::String(s)) => s.clone(),
        Some(o) => o.to_string(),
    }
}

fn board_data(m: &Map<String, Value>) -> BoardData {
    let sprints = m
        .get("sprints")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_object)
                .map(|s| SprintInfo {
                    id: integer(s, "id"),
                    end: text(s, "end").unwrap_or("").to_string(),
                    name: text(s, "name").map(String::from),
                    display_name: visible(s.get("name")),
                    board_id: s.get("boardId").and_then(Value::as_i64),
                    display_board_id: visible(s.get("boardId")),
                })
                .collect()
        })
        .unwrap_or_default();
    BoardData {
        project: text(m, "project").map(String::from),
        issue_type: text(m, "type").map(String::from),
        comp: text(m, "comp").map(String::from),
        subtask: m.get("subtask").and_then(Value::as_bool).unwrap_or(false),
        parent: non_empty_text(m, "parent").map(String::from),
        sprints,
    }
}

/// `Math.round((new Date(res) - new Date(due)) / DAY)`; `None` if a date is not understood (NaN).
fn days_late(res: &str, due: &str) -> Option<i64> {
    let (a, b) = (instant_ms(res)?, instant_ms(due)?);
    Some(js_round((a - b) as f64 / DAY_MS) as i64)
}

fn derive_worklog(w: &Value) -> AppResult<(Value, WorklogCalc)> {
    let mut map = w.as_object().cloned().ok_or_else(|| AppError::Format("Un registro de horas no es un objeto.".into()))?;
    let d = text(&map, "d").ok_or_else(|| AppError::Format("Un registro de horas no trae la fecha `d`.".into()))?;
    let day = local_date(d)?;
    let weekend = super::calendar::weekday_index(&day)? >= 5;
    let monday = monday_of(&day)?;
    let wk = if weekend { None } else { Some(monday.clone()) };
    map.insert("day".into(), day.clone().into());
    map.insert("weekend".into(), weekend.into());
    map.insert("monday".into(), monday.into());
    map.insert("wk".into(), wk.clone().map_or(Value::Null, Value::from));
    let s = integer(&map, "s");
    Ok((Value::Object(map), WorklogCalc { s, day, weekend, wk }))
}

/// Computes the derived fields of all issues, in the same order as the input.
pub fn derive(issues: &[Map<String, Value>]) -> AppResult<Vec<DerivedIssue>> {
    let data: Vec<BoardData> = issues.iter().map(board_data).collect();
    // `Object.fromEntries`: with repeated keys the last one wins.
    let mut by_key: HashMap<&str, &BoardData> = HashMap::new();
    for (t, d) in issues.iter().zip(&data) {
        if let Some(k) = text(t, "key") {
            by_key.insert(k, d);
        }
    }

    let mut output = Vec::with_capacity(issues.len());
    for (t, d) in issues.iter().zip(&data) {
        let mut fields = t.clone();
        let r = resolve_board(d, &by_key, 0);

        let mut wls = Vec::new();
        let mut wls_json = Vec::new();
        for w in t.get("worklogs").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default() {
            let (json, calc) = derive_worklog(w)?;
            wls_json.push(json);
            wls.push(calc);
        }
        if t.contains_key("worklogs") {
            fields.insert("worklogs".into(), Value::Array(wls_json));
        }

        let cat = text(t, "cat").map(String::from);
        let is_done = cat.as_deref() == Some("done");
        let fin_day = date_only(text(t, "finDev"))?;
        let res_day = match &fin_day {
            Some(f) => Some(f.clone()),
            None if is_done => date_only(non_empty_text(t, "resolved").or(text(t, "scd")))?,
            None => None,
        };
        let no_fin = fin_day.is_none() && is_done;
        let due = non_empty_text(t, "due").map(String::from);
        let late = match (&res_day, &due) {
            (Some(r), Some(d)) => days_late(r, d),
            _ => None,
        };
        let has_wl = !wls.is_empty();
        let need_doc = match t.get("hasPR") {
            Some(v) => v.clone(),
            None => Value::Bool(
                !no_doc_re().is_match(text(t, "summary").unwrap_or("")) && text(t, "type") != Some("Subtarea"),
            ),
        };
        let cre_day = date_only(text(t, "created"))?;

        let null_or = |o: &Option<String>| o.clone().map_or(Value::Null, Value::from);
        fields.insert("board".into(), r.board.clone().into());
        fields.insert("boardHow".into(), r.how.clone().into());
        fields.insert("finDay".into(), null_or(&fin_day));
        fields.insert("resDay".into(), null_or(&res_day));
        fields.insert("noFin".into(), no_fin.into());
        fields.insert("late".into(), late.map_or(Value::Null, Value::from));
        fields.insert("hasWl".into(), has_wl.into());
        fields.insert("needDoc".into(), need_doc.clone());
        fields.insert("creDay".into(), null_or(&cre_day));

        let c = Calc {
            key: text(t, "key").unwrap_or_default().to_string(),
            summary: text(t, "summary").unwrap_or_default().to_string(),
            status: text(t, "status").map(String::from),
            cat,
            est: integer(t, "est"),
            spent: integer(t, "spent"),
            due,
            updated: text(t, "updated").map(String::from),
            board: r.board,
            board_how: r.how,
            fin_day,
            res_day,
            no_fin,
            late,
            has_wl,
            need_doc,
            cre_day,
            worklogs: wls,
        };
        output.push(DerivedIssue { fields, c });
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn issue(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    fn one(v: Value) -> DerivedIssue {
        derive(&[issue(v)]).unwrap().remove(0)
    }

    #[test]
    fn issue_without_dates_or_hours() {
        let t = one(json!({ "key": "A-1", "summary": "Hacer algo", "type": "Tarea", "cat": "new", "worklogs": [], "est": 0, "spent": 0 }));
        assert_eq!(t.fields["finDay"], Value::Null);
        assert_eq!(t.fields["resDay"], Value::Null);
        assert_eq!(t.fields["creDay"], Value::Null);
        assert_eq!(t.fields["late"], Value::Null);
        assert_eq!(t.fields["noFin"], json!(false));
        assert_eq!(t.fields["hasWl"], json!(false));
        assert_eq!(t.fields["needDoc"], json!(true));
        assert_eq!(t.fields["board"], "Sin tablero");
    }

    #[test]
    fn resolution_and_lateness() {
        let t = one(json!({ "key": "A-1", "summary": "x", "cat": "done", "finDev": "2026-11-12T10:00:00.000-05:00",
            "due": "2026-11-10", "created": "2026-11-01T02:00:00.000Z", "worklogs": [] }));
        assert_eq!(t.fields["finDay"], "2026-11-12");
        assert_eq!(t.fields["resDay"], "2026-11-12");
        assert_eq!(t.fields["late"], json!(2));
        assert_eq!(t.fields["noFin"], json!(false));
        assert_eq!(t.fields["creDay"], "2026-10-31");

        // Without Fin desarrollo and concluded: uses resolved, or scd if there is none.
        let t = one(json!({ "key": "A-2", "cat": "done", "resolved": "2026-11-09T12:00:00.000-05:00", "scd": "2026-11-11", "due": "2026-11-12", "worklogs": [] }));
        assert_eq!(t.fields["resDay"], "2026-11-09");
        assert_eq!(t.fields["noFin"], json!(true));
        assert_eq!(t.fields["late"], json!(-3));
        let t = one(json!({ "key": "A-3", "cat": "done", "resolved": null, "scd": "2026-11-11", "worklogs": [] }));
        assert_eq!(t.fields["resDay"], "2026-11-11");
        // Open without Fin desarrollo: no resDay even if it has resolved.
        let t = one(json!({ "key": "A-4", "cat": "indeterminate", "resolved": "2026-11-11", "worklogs": [] }));
        assert_eq!(t.fields["resDay"], Value::Null);
    }

    #[test]
    fn weekend_worklogs_have_no_week() {
        let t = one(json!({ "key": "A-1", "worklogs": [
            { "d": "2026-11-13T09:00:00.000-05:00", "s": 3600, "a": "x" },
            { "d": "2026-11-14T09:00:00.000-05:00", "s": 1800, "a": null },
            { "d": "2026-11-15T03:00:00.000Z", "s": 60 }
        ]}));
        let w = t.fields["worklogs"].as_array().unwrap();
        assert_eq!(w[0]["day"], "2026-11-13");
        assert_eq!(w[0]["wk"], "2026-11-09");
        assert_eq!(w[0]["monday"], "2026-11-09");
        assert_eq!(w[0]["weekend"], json!(false));
        assert_eq!(w[0]["a"], "x");
        assert_eq!(w[1]["weekend"], json!(true));
        assert_eq!(w[1]["wk"], Value::Null);
        assert_eq!(w[1]["monday"], "2026-11-09");
        // 03:00 UTC on Sunday is Saturday night in Ecuador.
        assert_eq!(w[2]["day"], "2026-11-14");
        assert!(t.c.has_wl);
        assert!(t.c.worklogs[1].wk.is_none());
    }

    #[test]
    fn need_doc_by_text_type_and_has_pr() {
        let n = |v: Value| one(v).fields["needDoc"].clone();
        assert_eq!(n(json!({ "key": "A", "summary": "Reunión semanal", "worklogs": [] })), json!(false));
        assert_eq!(n(json!({ "key": "A", "summary": "REVISIÓN DE TAREA 5", "worklogs": [] })), json!(false));
        assert_eq!(n(json!({ "key": "A", "summary": "Análisis de datos", "worklogs": [] })), json!(false));
        assert_eq!(n(json!({ "key": "A", "summary": "Soporte a cliente", "worklogs": [] })), json!(false));
        assert_eq!(n(json!({ "key": "A", "summary": "Pantalla", "type": "Subtarea", "worklogs": [] })), json!(false));
        assert_eq!(n(json!({ "key": "A", "summary": "Pantalla", "type": "Tarea", "worklogs": [] })), json!(true));
        // hasPR rules when defined, even over the text and the type.
        assert_eq!(n(json!({ "key": "A", "summary": "Reunión", "hasPR": true, "worklogs": [] })), json!(true));
        assert_eq!(n(json!({ "key": "A", "summary": "Pantalla", "hasPR": false, "worklogs": [] })), json!(false));
    }

    #[test]
    fn keeps_unknown_fields() {
        let t = one(json!({ "key": "A", "docChecked": true, "extra": { "x": [1, 2] }, "worklogs": [{ "d": "2026-11-10", "s": 1, "otro": 5 }] }));
        assert_eq!(t.fields["docChecked"], json!(true));
        assert_eq!(t.fields["extra"], json!({ "x": [1, 2] }));
        assert_eq!(t.fields["worklogs"][0]["otro"], json!(5));
        // And the fields that were not there remain absent.
        assert!(!t.fields.contains_key("hasPR"));
        assert!(!t.fields.contains_key("sp"));
    }

    #[test]
    fn subtask_takes_the_parent_board() {
        let r = derive(&[
            issue(json!({ "key": "P-1", "project": "ADN", "worklogs": [] })),
            issue(json!({ "key": "P-2", "subtask": true, "parent": "P-1", "worklogs": [] })),
        ])
        .unwrap();
        assert_eq!(r[1].fields["board"], "Tablero ADN");
        assert_eq!(r[1].fields["boardHow"], "tarea padre P-1");
    }

    #[test]
    fn worklog_without_date_is_a_format_error() {
        let e = derive(&[issue(json!({ "key": "A", "worklogs": [{ "s": 5 }] }))]).unwrap_err();
        assert_eq!(e.code(), "format");
    }
}
