//! Data model received by the UI.
//!
//! Converts the JSON of the Jira REST API into the format the artifact already used
//! (the same field names: `key`, `summary`, `cat`, `finDev`, `worklogs`…), so the UI
//! code barely changes. Jira dates (`2026-11-10T08:30:00.000-0500`) become RFC 3339
//! (`…-05:00`), which every web engine interprets the same way.

use chrono::{DateTime, SecondsFormat};
use serde::Serialize;
use serde_json::Value;

/// Torres & Torres custom Jira fields.
pub const START_FIELD: &str = "customfield_10015";
pub const SPRINTS_FIELD: &str = "customfield_10020";
pub const POINTS_FIELD: &str = "customfield_10031";
pub const DEV_END_FIELD: &str = "customfield_10057";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub account_id: String,
    pub name: String,
    pub email: Option<String>,
}

impl User {
    /// From `GET /rest/api/3/myself`.
    pub fn from_jira(v: &Value) -> Option<User> {
        Some(User {
            account_id: v.get("accountId")?.as_str()?.to_string(),
            name: text(v, "/displayName").unwrap_or_default(),
            email: text(v, "/emailAddress"),
        })
    }
}

/// A worklog: start day (`d`), seconds (`s`) and author (`a`).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Worklog {
    pub d: String,
    pub s: i64,
    pub a: Option<String>,
}

impl Worklog {
    pub fn from_jira(v: &Value) -> Option<Worklog> {
        Some(Worklog {
            d: normalize_date(v.get("started")?.as_str()?),
            s: v.get("timeSpentSeconds").and_then(Value::as_i64).unwrap_or(0),
            a: text(v, "/author/accountId"),
        })
    }

    /// Only the user's own worklogs count. If the user or the author is unknown, it is kept.
    pub fn is_by(&self, me: Option<&str>) -> bool {
        match (me, self.a.as_deref()) {
            (Some(m), Some(a)) => a == m,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sprint {
    pub id: i64,
    pub name: String,
    pub board_id: Option<i64>,
    pub state: Option<String>,
    pub end: String,
}

impl Sprint {
    fn from_jira(v: &Value) -> Option<Sprint> {
        Some(Sprint {
            id: v.get("id")?.as_i64()?,
            name: text(v, "/name").unwrap_or_default(),
            board_id: v.get("boardId").and_then(Value::as_i64),
            state: text(v, "/state"),
            end: text(v, "/endDate").or_else(|| text(v, "/completeDate")).unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub key: String,
    pub summary: String,
    pub project: Option<String>,
    pub project_name: Option<String>,
    pub comp: String,
    #[serde(rename = "type")]
    pub issue_type: Option<String>,
    pub subtask: bool,
    pub status: Option<String>,
    pub cat: Option<String>,
    pub priority: Option<String>,
    pub scd: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub resolved: Option<String>,
    pub due: Option<String>,
    pub start: Option<String>,
    pub fin_dev: Option<String>,
    pub est: i64,
    pub spent: i64,
    pub sp: Option<f64>,
    pub labels: Vec<String>,
    pub sprints: Vec<Sprint>,
    pub sprint: Option<String>,
    pub parent: Option<String>,
    pub wl_total: i64,
    pub worklogs: Vec<Worklog>,
}

impl Issue {
    /// From an element of `issues` in `POST /rest/api/3/search/jql`.
    pub fn from_jira(v: &Value, me: Option<&str>) -> Option<Issue> {
        let key = v.get("key")?.as_str()?.to_string();
        let empty = Value::Null;
        let f = v.get("fields").unwrap_or(&empty);
        let date = |p: &str| text(f, p).map(|s| normalize_date(&s));
        let sprints: Vec<Sprint> = f
            .get(SPRINTS_FIELD)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Sprint::from_jira).collect())
            .unwrap_or_default();
        let worklogs: Vec<Worklog> = f
            .pointer("/worklog/worklogs")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Worklog::from_jira).filter(|r| r.is_by(me)).collect())
            .unwrap_or_default();
        Some(Issue {
            summary: text(f, "/summary").unwrap_or_default(),
            project: text(f, "/project/key"),
            project_name: text(f, "/project/name"),
            comp: text(f, "/components/0/name").unwrap_or_else(|| "Sin componente".into()),
            issue_type: text(f, "/issuetype/name"),
            subtask: f.pointer("/issuetype/subtask").and_then(Value::as_bool).unwrap_or(false),
            status: text(f, "/status/name"),
            cat: text(f, "/status/statusCategory/key"),
            priority: text(f, "/priority/name"),
            scd: date("/statuscategorychangedate"),
            created: date("/created"),
            updated: date("/updated"),
            resolved: date("/resolutiondate"),
            due: date("/duedate"),
            start: date(&format!("/{START_FIELD}")),
            fin_dev: date(&format!("/{DEV_END_FIELD}")),
            est: f.get("timeoriginalestimate").and_then(Value::as_i64).unwrap_or(0),
            spent: f.get("timespent").and_then(Value::as_i64).unwrap_or(0),
            sp: f.get(POINTS_FIELD).and_then(Value::as_f64),
            labels: f
                .get("labels")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|l| l.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            sprint: sprints.last().map(|s| s.name.clone()),
            sprints,
            parent: text(f, "/parent/key"),
            wl_total: f.pointer("/worklog/total").and_then(Value::as_i64).unwrap_or(0),
            worklogs,
            key,
        })
    }
}

fn text(v: &Value, pointer: &str) -> Option<String> {
    v.pointer(pointer).and_then(Value::as_str).map(String::from)
}

/// Jira date-time → RFC 3339. Dates without a time (`2026-11-10`) stay the same.
pub fn normalize_date(s: &str) -> String {
    DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f%z")
        .or_else(|_| DateTime::parse_from_rfc3339(s))
        .map(|d| d.to_rfc3339_opts(SecondsFormat::Millis, false))
        .unwrap_or_else(|_| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalizes_jira_dates() {
        assert_eq!(normalize_date("2026-11-10T08:30:00.000-0500"), "2026-11-10T08:30:00.000-05:00");
        assert_eq!(normalize_date("2026-11-10T08:30:00-05:00"), "2026-11-10T08:30:00.000-05:00");
        assert_eq!(normalize_date("2026-11-10"), "2026-11-10");
    }

    #[test]
    fn converts_issue_and_filters_foreign_worklogs() {
        let v = json!({
            "key": "APP-1",
            "fields": {
                "summary": "Prueba",
                "project": {"key": "APP", "name": "Aplicaciones"},
                "issuetype": {"name": "Tarea", "subtask": false},
                "status": {"name": "En curso", "statusCategory": {"key": "indeterminate"}},
                "created": "2026-11-01T10:00:00.000-0500",
                "duedate": "2026-11-20",
                "customfield_10057": "2026-11-12",
                "customfield_10020": [{"id": 7, "name": "AFO Sprint 23", "boardId": 455, "state": "active", "endDate": "2026-11-20T17:00:00.000Z"}],
                "timeoriginalestimate": 7200,
                "worklog": {"total": 2, "worklogs": [
                    {"started": "2026-11-10T08:30:00.000-0500", "timeSpentSeconds": 3600, "author": {"accountId": "yo"}},
                    {"started": "2026-11-10T09:30:00.000-0500", "timeSpentSeconds": 1800, "author": {"accountId": "otra"}}
                ]}
            }
        });
        let t = Issue::from_jira(&v, Some("yo")).unwrap();
        assert_eq!(t.key, "APP-1");
        assert_eq!(t.comp, "Sin componente");
        assert_eq!(t.cat.as_deref(), Some("indeterminate"));
        assert_eq!(t.created.as_deref(), Some("2026-11-01T10:00:00.000-05:00"));
        assert_eq!(t.fin_dev.as_deref(), Some("2026-11-12"));
        assert_eq!(t.sprint.as_deref(), Some("AFO Sprint 23"));
        assert_eq!(t.sprints[0].board_id, Some(455));
        assert_eq!(t.wl_total, 2);
        assert_eq!(t.worklogs.len(), 1);
        assert_eq!(t.worklogs[0].s, 3600);

        // The UI expects the artifact's names.
        let js = serde_json::to_value(&t).unwrap();
        for field in ["finDev", "projectName", "wlTotal", "type"] {
            assert!(js.get(field).is_some(), "falta {field}");
        }
        assert_eq!(js["sprints"][0]["boardId"], 455);
    }

    #[test]
    fn no_key_means_no_issue() {
        assert!(Issue::from_jira(&json!({"fields": {}}), None).is_none());
    }
}
