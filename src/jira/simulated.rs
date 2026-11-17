//! Fictitious data source to try the app without connecting to Jira.
//!
//! Deterministically generates, relative to `today`, about 18 issues with the same JSON shape as the
//! Jira REST API. This way the board always looks «alive» and `Issue::from_jira` is exercised too.
//! It covers the board cases: each board type, each status and each signal (overdue, past start,
//! finished late, without «Fin desarrollo», estimate exceeded, weekend, days with few hours,
//! deployment document present, missing or only as a remote link).
//!
//! Everything is made up: people, companies, issues and site (`ejemplo.atlassian.net`).

use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::Duration;

use chrono::{Datelike, Duration as Days, NaiveDate, Utc, Weekday};
use regex::Regex;
use serde_json::{json, Map, Value};

use super::model::User;
use super::{validate_key, RemoteLink, SearchPage, WikiPage};
use crate::error::{AppError, AppResult};

/// Same list as `BASE_HOLIDAYS` in `ui/app.js`: on those days the simulated source logs no hours.
pub const BASE_HOLIDAYS: [&str; 12] = [
    "2026-01-01",
    "2026-02-16",
    "2026-02-17",
    "2026-04-03",
    "2026-05-01",
    "2026-05-25",
    "2026-08-10",
    "2026-10-09",
    "2026-11-02",
    "2026-11-03",
    "2026-12-25",
    "2027-01-01",
];

const DELAY: Duration = Duration::from_millis(120);
const PAGE_SIZE: usize = 10;
const MAX_WORKLOGS_PER_ISSUE: usize = 20;
const ME: &str = "sim-0001";
const OTHER: &str = "sim-0002";
const SITE: &str = "https://ejemplo.atlassian.net";
/// Weeks of hours history (14 full weeks back from the current Monday).
const HISTORY_DAYS: i64 = 98;

/// Today's date in Ecuador (UTC-5, no daylight saving time).
pub fn today_ecuador() -> NaiveDate {
    (Utc::now() - Days::hours(5)).date_naive()
}

fn is_holiday(d: NaiveDate) -> bool {
    let s = d.format("%Y-%m-%d").to_string();
    BASE_HOLIDAYS.contains(&s.as_str())
}

fn is_workday(d: NaiveDate) -> bool {
    !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) && !is_holiday(d)
}

// ---------------------------------------------------------------------------------------------
// Issue definitions (dates as day offsets from the Monday of the current week)
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
enum Status {
    #[default]
    ToDo,
    InProgress,
    InTesting,
    Done,
}

impl Status {
    fn name_and_category(self) -> (&'static str, &'static str) {
        match self {
            Status::ToDo => ("Por hacer", "new"),
            Status::InProgress => ("En curso", "indeterminate"),
            Status::InTesting => ("En pruebas", "indeterminate"),
            Status::Done => ("Finalizada", "done"),
        }
    }
}

#[derive(Clone, Copy, Default)]
enum Estimate {
    #[default]
    Unset,
    /// Fixed hours.
    Hours(i64),
    /// Multiple of what was actually spent (below 1 simulates an exceeded estimate).
    Factor(f64),
}

#[derive(Clone, Copy, Default, PartialEq)]
enum DocKind {
    #[default]
    Missing,
    Confluence,
    LinkOnly,
}

#[derive(Clone, Copy)]
struct SprintDef {
    id: i64,
    name: &'static str,
    board: i64,
    status: &'static str,
    /// Closing day (offset from the current Monday).
    end: i64,
}

#[derive(Default)]
struct IssueDef {
    key: &'static str,
    summary: &'static str,
    status: Status,
    component: Option<&'static str>,
    sprints: &'static [SprintDef],
    /// `Some(key)` marks a subtask of that issue.
    subtask_of: Option<&'static str>,
    priority: &'static str,
    created: i64,
    due: Option<i64>,
    start: Option<i64>,
    dev_end: Option<i64>,
    /// Resolution when there is no `dev_end` («finished without Fin desarrollo» case).
    resolved: Option<i64>,
    /// Days (inclusive) on which own hours are logged; `None` if the issue has no hours.
    window: Option<(i64, i64)>,
    estimate: Estimate,
    points: Option<f64>,
    labels: &'static [&'static str],
    pr: bool,
    doc: DocKind,
}

const SPRINT_AFO_22: SprintDef = SprintDef { id: 701, name: "AFO Sprint 22", board: 455, status: "closed", end: -10 };
const SPRINT_AFO_23: SprintDef = SprintDef { id: 702, name: "AFO Sprint 23", board: 455, status: "active", end: 4 };
const SPRINT_DAI_15: SprintDef = SprintDef { id: 801, name: "DAI Sprint 15", board: 92, status: "closed", end: -3 };

const AFO_101_SPRINTS: &[SprintDef] = &[SPRINT_AFO_23];
const AFO_102_SPRINTS: &[SprintDef] = &[SPRINT_AFO_22, SPRINT_AFO_23];
const DAI_SPRINTS: &[SprintDef] = &[SPRINT_DAI_15];

fn definitions() -> Vec<IssueDef> {
    vec![
        // --- Sprint board: AFO (455) ---
        IssueDef {
            key: "AFO-101",
            summary: "Ajuste del cálculo de tributos en la liquidación de aforo",
            status: Status::InProgress,
            sprints: AFO_101_SPRINTS,
            priority: "Alta",
            created: -40,
            due: Some(6),
            start: Some(-35),
            window: Some((-35, 4)),
            estimate: Estimate::Factor(0.6), // exceeds the estimate by more than 20 %
            points: Some(8.0),
            labels: &["aforo", "tributos"],
            pr: true,
            doc: DocKind::Confluence,
            ..IssueDef::default()
        },
        IssueDef {
            key: "AFO-102",
            summary: "Validación de partidas arancelarias en la declaración aduanera",
            status: Status::InTesting,
            sprints: AFO_102_SPRINTS,
            priority: "Media",
            created: -30,
            due: Some(8),
            start: Some(-21),
            dev_end: Some(-1),
            window: Some((-21, 4)),
            estimate: Estimate::Factor(1.1),
            points: Some(5.0),
            pr: true,
            doc: DocKind::Confluence,
            ..IssueDef::default()
        },
        // Subtask: the board is resolved through the parent issue.
        IssueDef {
            key: "AFO-103",
            summary: "Pruebas unitarias del cálculo de tributos",
            status: Status::InProgress,
            subtask_of: Some("AFO-101"),
            priority: "Media",
            created: -12,
            due: Some(5),
            start: Some(-10),
            window: Some((-10, 4)),
            estimate: Estimate::Factor(1.15),
            ..IssueDef::default()
        },
        // --- Sprint board: DAI (92) ---
        IssueDef {
            key: "DAI-201",
            summary: "Integración del manifiesto de carga con el sistema aduanero",
            status: Status::Done,
            sprints: DAI_SPRINTS,
            priority: "Alta",
            created: -55,
            due: Some(-6),
            start: Some(-49),
            dev_end: Some(-4), // finished 2 days after the due date
            window: Some((-49, -4)),
            estimate: Estimate::Factor(1.1),
            points: Some(13.0),
            pr: true,
            doc: DocKind::Confluence,
            ..IssueDef::default()
        },
        IssueDef {
            key: "DAI-202",
            summary: "Reporte de declaraciones pendientes por distrito aduanero",
            status: Status::Done,
            sprints: DAI_SPRINTS,
            priority: "Media",
            created: -48,
            due: Some(-8),
            start: Some(-42),
            resolved: Some(-9), // finished WITHOUT Fecha Fin Desarrollo
            window: Some((-42, -10)),
            estimate: Estimate::Factor(1.1),
            pr: true,
            doc: DocKind::Confluence,
            ..IssueDef::default()
        },
        // --- Project board: ADN ---
        IssueDef {
            key: "ADN-301",
            summary: "Migración de la plantilla de notificaciones a clientes",
            status: Status::ToDo, // its start date has passed and it is still «Por hacer»
            priority: "Baja",
            created: -14,
            due: Some(10),
            start: Some(-5),
            estimate: Estimate::Hours(8),
            ..IssueDef::default()
        },
        IssueDef {
            key: "ADN-302",
            summary: "Conciliación automática de guías de carga",
            status: Status::InProgress,
            priority: "Alta",
            created: -35,
            due: Some(-3), // open and overdue
            start: Some(-28),
            window: Some((-28, 4)),
            estimate: Estimate::Factor(1.1),
            pr: true,
            doc: DocKind::Confluence,
            ..IssueDef::default()
        },
        // --- Kanban board filter (APP/GPS projects, Tarea type) ---
        IssueDef {
            key: "APP-401",
            summary: "Automatización del envío de documentos a bodega",
            status: Status::InProgress,
            priority: "Media",
            created: -20,
            due: Some(12),
            start: Some(-14),
            window: Some((-14, 4)),
            estimate: Estimate::Factor(1.2),
            pr: true,
            doc: DocKind::Confluence,
            ..IssueDef::default()
        },
        // The deployment document only exists as a remote link.
        IssueDef {
            key: "APP-402",
            summary: "Notificación automática de arribo de contenedores",
            status: Status::Done,
            priority: "Media",
            created: -70,
            due: Some(-4),
            start: Some(-63),
            dev_end: Some(-5),
            window: Some((-63, -5)),
            estimate: Estimate::Factor(1.1),
            pr: true,
            doc: DocKind::LinkOnly,
            ..IssueDef::default()
        },
        // Recently finished, with a PR and WITHOUT a deployment document.
        IssueDef {
            key: "APP-403",
            summary: "Cálculo de recargos por almacenaje en patio",
            status: Status::Done,
            priority: "Alta",
            created: -32,
            due: Some(-3),
            start: Some(-28),
            dev_end: Some(-3),
            window: Some((-28, -3)),
            estimate: Estimate::Factor(1.1),
            pr: true,
            doc: DocKind::Missing,
            ..IssueDef::default()
        },
        IssueDef {
            key: "APP-404",
            summary: "Conciliación de facturas de transporte terrestre",
            status: Status::ToDo, // starts today
            priority: "Baja",
            created: -5,
            due: Some(15),
            start: Some(i64::MIN), // marker: resolved to «today» when generating
            estimate: Estimate::Hours(6),
            ..IssueDef::default()
        },
        IssueDef {
            key: "GPS-501",
            summary: "Alerta de vencimiento de garantías de transporte",
            status: Status::Done,
            priority: "Media",
            created: -85,
            due: Some(-7),
            start: Some(-77),
            dev_end: Some(-6),
            window: Some((-77, -6)),
            estimate: Estimate::Factor(1.1),
            ..IssueDef::default()
        },
        // --- Backlog by component ---
        IssueDef {
            key: "OPE-601",
            summary: "Revisión del checklist de inspección de contenedores",
            status: Status::InProgress,
            component: Some("Aforo"),
            priority: "Media",
            created: -60,
            due: Some(9),
            start: Some(-56),
            window: Some((-56, 4)),
            estimate: Estimate::Factor(1.1),
            ..IssueDef::default()
        },
        IssueDef {
            key: "OPE-602",
            summary: "Parametrización de reglas para el régimen de consumo",
            status: Status::InProgress,
            component: Some("DAI"),
            priority: "Media",
            created: -33,
            due: Some(11),
            start: Some(-30),
            window: Some((-30, 4)),
            estimate: Estimate::Factor(1.1),
            ..IssueDef::default()
        },
        IssueDef {
            key: "OPE-603",
            summary: "Control de saldos de depósito temporal",
            status: Status::Done,
            component: Some("Regímenes Especiales"),
            priority: "Baja",
            created: -95,
            due: Some(-30),
            start: Some(-91),
            dev_end: Some(-31),
            window: Some((-91, -31)),
            estimate: Estimate::Factor(1.1),
            ..IssueDef::default()
        },
        // --- Meeting, support and training (no PR, no board) ---
        IssueDef {
            key: "OPE-701",
            summary: "Reunión semanal de seguimiento de operaciones",
            status: Status::InProgress,
            priority: "Baja",
            created: -105,
            window: Some((-105, 4)),
            ..IssueDef::default()
        },
        IssueDef {
            key: "OPE-702",
            summary: "Soporte a usuarios de la plataforma de operaciones",
            status: Status::InProgress,
            priority: "Media",
            created: -105,
            window: Some((-105, 4)),
            estimate: Estimate::Factor(1.3),
            ..IssueDef::default()
        },
        IssueDef {
            key: "OPE-703",
            summary: "Capacitación en el nuevo módulo de regímenes especiales",
            status: Status::Done,
            priority: "Baja",
            created: -50,
            due: Some(-34),
            start: Some(-45),
            dev_end: Some(-35),
            window: Some((-45, -35)),
            estimate: Estimate::Factor(1.1),
            ..IssueDef::default()
        },
    ]
}

// ---------------------------------------------------------------------------------------------
// Worklogs
// ---------------------------------------------------------------------------------------------

#[derive(Clone)]
struct SimWorklog {
    id: u64,
    author: &'static str,
    day: NaiveDate,
    hour: u32,
    seconds: i64,
}

impl SimWorklog {
    /// `2026-11-10T08:30:00.000-0500`
    fn jira_started(&self) -> String {
        format!("{}T{:02}:30:00.000-0500", self.day.format("%Y-%m-%d"), self.hour)
    }

    fn started_ms(&self) -> i64 {
        let local = self.day.and_hms_opt(self.hour, 30, 0).expect("valid time");
        local.and_utc().timestamp_millis() + 5 * 3_600_000
    }

    fn to_json(&self) -> Value {
        json!({
            "id": self.id.to_string(),
            "author": { "accountId": self.author, "displayName": if self.author == ME { "Persona de Prueba" } else { "Otra Persona" } },
            "started": self.jira_started(),
            "timeSpentSeconds": self.seconds,
            "timeSpent": duration_text(self.seconds),
        })
    }
}

fn duration_text(secs: i64) -> String {
    let (h, m) = (secs / 3600, (secs % 3600) / 60);
    match (h, m) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}

/// Spreads the own hours of each workday across 1 or 2 active issues.
fn generate_worklogs(defs: &[IssueDef], monday: NaiveDate, today: NaiveDate) -> Vec<Vec<SimWorklog>> {
    let mut per_issue: Vec<Vec<SimWorklog>> = vec![Vec::new(); defs.len()];
    let mut next_id: u64 = 100_000;
    let mut new_worklog = |author, day, hour, seconds| {
        next_id += 1;
        SimWorklog { id: next_id, author, day, hour, seconds }
    };

    // In the current week: the first workday already past gets 0 h and the second one 4 h.
    let past_week_days: Vec<NaiveDate> =
        (0..5).map(|n| monday + Days::days(n)).filter(|d| *d < today && is_workday(*d)).collect();

    let mut day = monday - Days::days(HISTORY_DAYS);
    while day < today {
        if is_workday(day) {
            let total: i64 = match past_week_days.iter().position(|d| *d == day) {
                Some(0) => 0,
                Some(1) => 4 * 3600,
                _ => 8 * 3600,
            };
            let offset = (day - monday).num_days();
            let active: Vec<usize> = defs
                .iter()
                .enumerate()
                .filter(|(_, d)| d.window.is_some_and(|(a, b)| offset >= a && offset <= b))
                .map(|(i, _)| i)
                .collect();
            if total > 0 && !active.is_empty() {
                let order = day.num_days_from_ce() as usize;
                let n = active.len();
                // The active issue with the fewest worklogs so far is chosen (so all of them get hours).
                let least_loaded = |exclude: Option<usize>| {
                    active.iter().copied().filter(|i| Some(*i) != exclude).min_by_key(|i| (per_issue[*i].len(), *i))
                };
                let a = least_loaded(None).expect("there are active issues");
                if n == 1 || order % 3 == 0 {
                    per_issue[a].push(new_worklog(ME, day, 8, total));
                } else {
                    // Recurring support is always the second share: it accumulates more than 20 worklogs.
                    let support = active.iter().copied().find(|i| defs[*i].key == "OPE-702");
                    let b = support.filter(|i| *i != a).or_else(|| least_loaded(Some(a))).unwrap_or(a);
                    let larger = total * 5 / 8;
                    per_issue[a].push(new_worklog(ME, day, 8, larger));
                    per_issue[b].push(new_worklog(ME, day, 13, total - larger));
                }
            }
        }
        day += Days::days(1);
    }

    let index_of = |key: &str| defs.iter().position(|d| d.key == key).expect("issue defined");
    // Saturday hours (falls in the previous week): outside the working week.
    per_issue[index_of("AFO-101")].push(new_worklog(ME, monday - Days::days(2), 9, 3 * 3600));
    // Hours of another person on a shared issue: the UI must ignore them.
    for offset in [-6, -5] {
        let d = monday + Days::days(offset);
        per_issue[index_of("APP-401")].push(new_worklog(OTHER, d, 10, 2 * 3600));
    }
    for worklogs in &mut per_issue {
        worklogs.sort_by_key(|r| (r.day, r.hour, r.id));
    }
    per_issue
}

// ---------------------------------------------------------------------------------------------
// Simulated source
// ---------------------------------------------------------------------------------------------

struct SimIssue {
    key: &'static str,
    summary: &'static str,
    /// Jira fields, except `worklog` (built when answering to respect the cap of 20).
    fields: Map<String, Value>,
    worklogs: Vec<SimWorklog>,
    pr: bool,
    doc: DocKind,
}

impl SimIssue {
    fn issue(&self, fields: &[&str]) -> Value {
        let mut out = Map::new();
        for c in fields {
            let value = if *c == "worklog" {
                let first: Vec<Value> =
                    self.worklogs.iter().take(MAX_WORKLOGS_PER_ISSUE).map(SimWorklog::to_json).collect();
                json!({
                    "startAt": 0,
                    "maxResults": MAX_WORKLOGS_PER_ISSUE,
                    "total": self.worklogs.len(),
                    "worklogs": first,
                })
            } else {
                self.fields.get(*c).cloned().unwrap_or(Value::Null)
            };
            out.insert((*c).to_string(), value);
        }
        let id = 10_000 + self.key.rsplit('-').next().and_then(|n| n.parse::<u64>().ok()).unwrap_or(0);
        json!({ "id": id.to_string(), "key": self.key, "fields": out })
    }

    fn has_own_hours_since(&self, since: Option<NaiveDate>) -> bool {
        self.worklogs.iter().any(|r| r.author == ME && since.map_or(true, |d| r.day >= d))
    }

    fn wiki_page(&self, position: usize) -> WikiPage {
        WikiPage {
            title: format!("Documento de Despliegue - {} - {}", self.key, self.summary),
            url: format!("{SITE}/wiki/spaces/DEV/pages/{}/Documento+de+Despliegue", 9_000 + position),
        }
    }
}

pub struct SimulatedJira {
    email: String,
    valid_token: bool,
    issues: Vec<SimIssue>,
}

fn project_name(prefix: &str) -> &'static str {
    match prefix {
        "AFO" => "Aforo Digital",
        "DAI" => "Declaración Aduanera",
        "ADN" => "Analítica y Datos",
        "APP" => "Aplicaciones Internas",
        "GPS" => "Rastreo de Flota",
        _ => "Operaciones",
    }
}

fn date_time(d: NaiveDate, hour: u32) -> String {
    format!("{}T{:02}:00:00.000-0500", d.format("%Y-%m-%d"), hour)
}

impl SimulatedJira {
    pub fn new(email: &str, valid_token: bool, today: NaiveDate) -> Self {
        let monday = today - Days::days(i64::from(today.weekday().num_days_from_monday()));
        let defs = definitions();
        let worklogs = generate_worklogs(&defs, monday, today);
        let day = |offset: i64| {
            // `APP-404`'s «today» marker.
            if offset == i64::MIN {
                today
            } else {
                monday + Days::days(offset)
            }
        };
        let issues = defs
            .iter()
            .zip(worklogs)
            .map(|(d, wls)| {
                let (status, category) = d.status.name_and_category();
                let prefix = d.key.split('-').next().unwrap_or("");
                let spent: i64 = wls.iter().map(|r| r.seconds).sum();
                let estimated: i64 = match d.estimate {
                    Estimate::Unset => 0,
                    Estimate::Hours(h) => h * 3600,
                    Estimate::Factor(f) => ((spent as f64 * f / 3600.0).ceil() as i64) * 3600,
                };
                let resolution = d.dev_end.or(d.resolved).map(day);
                let is_subtask = d.subtask_of.is_some();
                let mut c = Map::new();
                c.insert("summary".into(), json!(d.summary));
                c.insert("status".into(), json!({ "name": status, "statusCategory": { "key": category } }));
                c.insert("project".into(), json!({ "key": prefix, "name": project_name(prefix) }));
                c.insert(
                    "issuetype".into(),
                    json!({ "name": if is_subtask { "Subtarea" } else { "Tarea" }, "subtask": is_subtask }),
                );
                c.insert("priority".into(), json!({ "name": d.priority }));
                c.insert("created".into(), json!(date_time(day(d.created), 9)));
                let updated = match (d.status, resolution) {
                    (Status::Done, Some(r)) => date_time(r, 16),
                    _ => date_time(today, 10),
                };
                c.insert("updated".into(), json!(updated));
                let status_change = resolution.unwrap_or_else(|| day(d.created) + Days::days(1));
                c.insert("statuscategorychangedate".into(), json!(date_time(status_change, 11)));
                if let (Status::Done, Some(r)) = (d.status, resolution) {
                    c.insert("resolutiondate".into(), json!(date_time(r, 16)));
                }
                if let Some(v) = d.due {
                    c.insert("duedate".into(), json!(day(v).format("%Y-%m-%d").to_string()));
                }
                if let Some(i) = d.start {
                    c.insert("customfield_10015".into(), json!(day(i).format("%Y-%m-%d").to_string()));
                }
                if let Some(f) = d.dev_end {
                    c.insert("customfield_10057".into(), json!(day(f).format("%Y-%m-%d").to_string()));
                }
                c.insert("timeoriginalestimate".into(), if estimated > 0 { json!(estimated) } else { Value::Null });
                c.insert("timespent".into(), if spent > 0 { json!(spent) } else { Value::Null });
                c.insert("components".into(), json!(d.component.map(|n| vec![json!({ "name": n })]).unwrap_or_default()));
                c.insert("labels".into(), json!(d.labels));
                if let Some(p) = d.points {
                    c.insert("customfield_10031".into(), json!(p));
                }
                if !d.sprints.is_empty() {
                    let sprints: Vec<Value> = d
                        .sprints
                        .iter()
                        .map(|s| {
                            let end = day(s.end);
                            json!({
                                "id": s.id,
                                "name": s.name,
                                "state": s.status,
                                "boardId": s.board,
                                "startDate": format!("{}T13:00:00.000Z", (end - Days::days(13)).format("%Y-%m-%d")),
                                "endDate": format!("{}T22:00:00.000Z", end.format("%Y-%m-%d")),
                            })
                        })
                        .collect();
                    c.insert("customfield_10020".into(), Value::Array(sprints));
                }
                if let Some(parent) = d.subtask_of {
                    c.insert("parent".into(), json!({ "key": parent }));
                }
                SimIssue { key: d.key, summary: d.summary, fields: c, worklogs: wls, pr: d.pr, doc: d.doc }
            })
            .collect();
        SimulatedJira { email: email.to_string(), valid_token, issues }
    }

    /// Waits a little (so loading states are visible) and checks the token.
    async fn serve(&self) -> AppResult<()> {
        tokio::time::sleep(DELAY).await;
        if self.valid_token {
            Ok(())
        } else {
            Err(AppError::Unauthorized)
        }
    }

    pub async fn myself(&self) -> AppResult<User> {
        self.serve().await?;
        Ok(User {
            account_id: ME.to_string(),
            name: "Persona de Prueba".to_string(),
            email: Some(self.email.clone()),
        })
    }

    /// Interprets the JQL by its content: with `development[pullrequests]` only the issues with a PR;
    /// with `worklogDate`, those with own hours; otherwise, all of them. Pages of 10.
    pub async fn search(
        &self,
        jql: &str,
        fields: &[&str],
        next: Option<&str>,
        _size: u32,
    ) -> AppResult<SearchPage> {
        self.serve().await?;
        let since = if jql.contains("worklogDate") {
            Some(
                jql.split_once("worklogDate >= \"")
                    .and_then(|(_, rest)| rest.get(..10))
                    .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
            )
        } else {
            None
        };
        let chosen: Vec<&SimIssue> = self
            .issues
            .iter()
            .filter(|t| {
                if jql.contains("development[pullrequests]") {
                    t.pr
                } else if let Some(d) = since {
                    t.has_own_hours_since(d)
                } else {
                    true
                }
            })
            .collect();
        let start = match next {
            None => 0,
            Some(t) => t
                .strip_prefix("sim-pag-")
                .and_then(|n| n.parse::<usize>().ok())
                .ok_or_else(|| AppError::Format("token de página no válido".into()))?,
        };
        let end = (start + PAGE_SIZE).min(chosen.len());
        let issues = chosen.get(start..end).unwrap_or(&[]).iter().map(|t| t.issue(fields)).collect();
        let next = (end < chosen.len()).then(|| format!("sim-pag-{end}"));
        Ok(SearchPage { issues, next })
    }

    /// All the issue's worklogs (from any author) started since `since_ms`.
    pub async fn worklogs(&self, key: &str, since_ms: i64) -> AppResult<Vec<Value>> {
        let key = validate_key(key)?;
        self.serve().await?;
        Ok(self
            .issues
            .iter()
            .find(|t| t.key == key)
            .map(|t| t.worklogs.iter().filter(|r| r.started_ms() >= since_ms).map(SimWorklog::to_json).collect())
            .unwrap_or_default())
    }

    /// Returns the «Documento de Despliegue» page of the issues with a document in Confluence
    /// whose key appears in the CQL.
    pub async fn confluence(&self, cql: &str) -> AppResult<Vec<WikiPage>> {
        self.serve().await?;
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| Regex::new(r"[A-Z][A-Z0-9_]*-[0-9]+").expect("valid regex"));
        let requested: HashSet<&str> = re.find_iter(cql).map(|m| m.as_str()).collect();
        Ok(self
            .issues
            .iter()
            .enumerate()
            .filter(|(_, t)| t.doc == DocKind::Confluence && requested.contains(t.key))
            .map(|(i, t)| t.wiki_page(i))
            .collect())
    }

    pub async fn links(&self, key: &str) -> AppResult<Vec<RemoteLink>> {
        let key = validate_key(key)?;
        self.serve().await?;
        let Some((i, t)) = self.issues.iter().enumerate().find(|(_, t)| t.key == key) else {
            return Ok(Vec::new());
        };
        let mut links = Vec::new();
        if t.pr {
            links.push(RemoteLink {
                title: format!("Pull request de {}", t.key),
                url: format!("https://repositorio.ejemplo.test/pulls/{}", 100 + i),
            });
        }
        if t.doc == DocKind::LinkOnly {
            let mut p = t.wiki_page(i);
            p.url = format!("{SITE}/wiki/spaces/DEV/pages/{}/Documento+de+Despliegue+{}", 9_000 + i, t.key);
            links.push(RemoteLink { title: p.title, url: p.url });
        }
        Ok(links)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jira::{FIELDS, ISSUES_JQL};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
    }

    async fn all(s: &SimulatedJira) -> Vec<Value> {
        let mut out = Vec::new();
        let mut token: Option<String> = None;
        loop {
            let p = s.search(ISSUES_JQL, FIELDS, token.as_deref(), 100).await.unwrap();
            out.extend(p.issues);
            token = p.next;
            if token.is_none() {
                return out;
            }
        }
    }

    #[tokio::test]
    async fn is_deterministic_for_a_fixed_today() {
        let a = all(&SimulatedJira::new("a@b.com", true, today())).await;
        let b = all(&SimulatedJira::new("otro@b.com", true, today())).await;
        assert_eq!(a, b);
        assert!(a.len() >= 17);
        let other_day = all(&SimulatedJira::new("a@b.com", true, today() + Days::days(7))).await;
        assert_ne!(a, other_day);
    }

    #[tokio::test]
    async fn no_hours_on_holidays_or_weekends_except_the_deliberate_saturday() {
        let s = SimulatedJira::new("a@b.com", true, today());
        let mut saturdays = 0;
        let mut days_with_hours: HashSet<NaiveDate> = HashSet::new();
        for t in &s.issues {
            for r in t.worklogs.iter().filter(|r| r.author == ME) {
                assert!(!is_holiday(r.day), "{} logs hours on holiday {}", t.key, r.day);
                if matches!(r.day.weekday(), Weekday::Sat | Weekday::Sun) {
                    saturdays += 1;
                    assert_eq!(t.key, "AFO-101");
                } else {
                    days_with_hours.insert(r.day);
                }
            }
        }
        assert_eq!(saturdays, 1);
        // November 2 and 3 are holidays and have no hours; a normal workday does.
        assert!(!days_with_hours.contains(&NaiveDate::from_ymd_opt(2026, 11, 2).unwrap()));
        assert!(days_with_hours.contains(&NaiveDate::from_ymd_opt(2026, 11, 4).unwrap()));
    }

    #[tokio::test]
    async fn normal_days_add_up_to_eight_hours_and_the_current_week_has_gaps() {
        let s = SimulatedJira::new("a@b.com", true, today() + Days::days(3)); // Thursday Nov 19
        let sum = |d: &str| -> i64 {
            let d = NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap();
            s.issues.iter().flat_map(|t| &t.worklogs).filter(|r| r.author == ME && r.day == d).map(|r| r.seconds).sum()
        };
        assert_eq!(sum("2026-11-10"), 8 * 3600);
        assert_eq!(sum("2026-11-16"), 0); // current Monday: 0 h
        assert_eq!(sum("2026-11-17"), 4 * 3600); // current Tuesday: less than 8 h
        assert_eq!(sum("2026-11-18"), 8 * 3600);
    }

    #[tokio::test]
    async fn embedded_worklog_is_capped_at_20_with_real_total() {
        let s = SimulatedJira::new("a@b.com", true, today());
        let all = all(&s).await;
        let base = all.iter().find(|v| v["key"] == "OPE-702").unwrap();
        let wl = &base["fields"]["worklog"];
        assert_eq!(wl["worklogs"].as_array().unwrap().len(), 20);
        assert!(wl["total"].as_u64().unwrap() > 20);
        let full = s.worklogs("OPE-702", 0).await.unwrap();
        assert_eq!(full.len() as u64, wl["total"].as_u64().unwrap());
        // `since_ms` cuts by start date.
        let recent = s.worklogs("OPE-702", 1_794_000_000_000).await.unwrap(); // approx. 2026-11-07
        assert!(!recent.is_empty() && recent.len() < full.len());
        assert!(s.worklogs("ZZZ-1", 0).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn json_shape_is_jira_and_covers_the_board_cases() {
        use crate::jira::model::Issue;
        let s = SimulatedJira::new("a@b.com", true, today());
        let issues: Vec<Issue> =
            all(&s).await.iter().map(|v| Issue::from_jira(v, Some(ME)).unwrap()).collect();
        let by = |k: &str| issues.iter().find(|t| t.key == k).unwrap();
        assert_eq!(by("AFO-101").sprints[0].board_id, Some(455));
        assert_eq!(by("DAI-201").sprints[0].board_id, Some(92));
        assert!(by("AFO-103").subtask && by("AFO-103").parent.as_deref() == Some("AFO-101"));
        assert_eq!(by("OPE-602").comp, "DAI");
        assert_eq!(by("OPE-701").comp, "Sin componente");
        assert_eq!(by("ADN-301").cat.as_deref(), Some("new"));
        assert_eq!(by("AFO-102").status.as_deref(), Some("En pruebas"));
        assert_eq!(by("DAI-202").cat.as_deref(), Some("done"));
        assert!(by("DAI-202").fin_dev.is_none() && by("DAI-202").resolved.is_some());
        assert!(by("DAI-201").fin_dev.is_some());
        let a = by("AFO-101");
        assert!(a.spent as f64 > a.est as f64 * 1.2, "must exceed the estimate");
        // Finished after the due date.
        assert!(by("DAI-201").fin_dev.as_deref().unwrap() > by("DAI-201").due.as_deref().unwrap());
        // Open and overdue.
        assert!(by("ADN-302").due.as_deref().unwrap() < "2026-11-16" && by("ADN-302").resolved.is_none());
        // Dates in RFC 3339 format after normalization.
        assert!(a.created.as_deref().unwrap().ends_with("-05:00"));
    }

    #[test]
    fn generates_without_errors_for_any_day_of_the_week() {
        for n in 0..21 {
            let s = SimulatedJira::new("a@b.com", true, today() + Days::days(n));
            assert_eq!(s.issues.len(), definitions().len());
        }
    }

    #[test]
    fn every_issue_with_a_window_has_own_hours() {
        let s = SimulatedJira::new("a@b.com", true, today());
        for (d, t) in definitions().iter().zip(&s.issues) {
            assert_eq!(
                d.window.is_some(),
                t.has_own_hours_since(None),
                "{}: la ventana de horas y los registros no coinciden",
                t.key
            );
        }
    }

    #[tokio::test]
    async fn invalid_token_rejects_all_calls() {
        let s = SimulatedJira::new("a@b.com", false, today());
        assert!(matches!(s.myself().await, Err(AppError::Unauthorized)));
        assert!(matches!(s.search("x", &[], None, 10).await, Err(AppError::Unauthorized)));
        assert!(matches!(s.worklogs("APP-1", 0).await, Err(AppError::Unauthorized)));
    }

    #[tokio::test]
    async fn invalid_page_token_is_a_format_error() {
        let s = SimulatedJira::new("a@b.com", true, today());
        assert!(matches!(s.search(ISSUES_JQL, FIELDS, Some("xx"), 10).await, Err(AppError::Format(_))));
    }

    #[test]
    fn today_ecuador_is_a_date_close_to_utc() {
        let diff = (Utc::now().date_naive() - today_ecuador()).num_days();
        assert!((0..=1).contains(&diff));
    }
}
