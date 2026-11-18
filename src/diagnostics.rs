//! Anonymous diagnostics: runs the load walk against the source, times each step and reports the
//! shape of the data without copying any value from Jira.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use chrono::NaiveDate;
use regex::{Captures, Regex};
use serde::{Serialize, Serializer};
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::jira::{self, Source};

const MAX_PAGES: usize = 20;
const MAX_SHAPE_DEPTH: usize = 4;
const MAX_MESSAGE_CHARS: usize = 300;
const MIN_KNOWN_CHARS: usize = 3;

/// What the session knows about the person; none of it may reach the report.
pub struct SessionFacts {
    pub site: String,
    pub email: String,
    pub name: String,
    pub account_id: String,
}

/// Values to hide from messages, plus the stable `ISSUE-N` numbering of the whole report.
pub struct Known {
    replacements: Vec<(String, &'static str)>,
    keys: Mutex<HashMap<String, usize>>,
}

impl Known {
    pub fn new(facts: &SessionFacts, issues: &[Value]) -> Self {
        let mut replacements: Vec<(String, &'static str)> = Vec::new();
        let host = facts.site.split_once("://").map_or(facts.site.as_str(), |(_, rest)| rest);
        replacements.push((facts.site.clone(), "[sitio]"));
        replacements.push((host.trim_end_matches('/').to_string(), "[sitio]"));
        replacements.push((facts.email.clone(), "[correo]"));
        replacements.push((facts.name.clone(), "[persona]"));
        replacements.push((facts.account_id.clone(), "[cuenta]"));
        for issue in issues {
            if let Some(title) = issue.pointer("/fields/summary").and_then(Value::as_str) {
                replacements.push((title.to_string(), "[título]"));
            }
        }
        replacements.retain(|(value, _)| value.chars().count() >= MIN_KNOWN_CHARS);
        replacements.sort_by_key(|r| std::cmp::Reverse(r.0.chars().count()));
        replacements.dedup_by(|a, b| a.0 == b.0);
        Known { replacements, keys: Mutex::new(HashMap::new()) }
    }

    fn issue_label(&self, key: &str) -> String {
        let mut keys = self.keys.lock().unwrap_or_else(|e| e.into_inner());
        let next = keys.len() + 1;
        format!("ISSUE-{}", keys.entry(key.to_string()).or_insert(next))
    }
}

fn url_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"https?://\S+").expect("valid regex"))
}

fn email_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").expect("valid regex"))
}

fn issue_key_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b[A-Z][A-Z0-9_]{0,19}-\d{1,9}\b").expect("valid regex"))
}

fn quoted_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"'([^']*)'|"([^"]*)"|«([^»]*)»"#).expect("valid regex"))
}

fn keeps_quoted(content: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^(customfield_\d+|ISSUE-\d+|\[[^\[\]]*\])$").expect("valid regex"));
    content.is_empty() || jira::FIELDS.contains(&content) || re.is_match(content)
}

/// Replaces anything that could identify a person, a site or an issue. The only free text in the report.
pub fn anonymize(text: &str, known: &Known) -> String {
    let mut out = text.to_string();
    for (value, marker) in &known.replacements {
        out = out.replace(value.as_str(), marker);
    }
    out = url_regex().replace_all(&out, "[url]").into_owned();
    out = email_regex().replace_all(&out, "[correo]").into_owned();
    out = issue_key_regex().replace_all(&out, |c: &Captures| known.issue_label(&c[0])).into_owned();
    out = quoted_regex()
        .replace_all(&out, |c: &Captures| {
            let (open, close, content) = if let Some(m) = c.get(1) {
                ("'", "'", m.as_str())
            } else if let Some(m) = c.get(2) {
                ("\"", "\"", m.as_str())
            } else {
                ("«", "»", c.get(3).map_or("", |m| m.as_str()))
            };
            if keeps_quoted(content) {
                c[0].to_string()
            } else {
                format!("{open}[texto]{close}")
            }
        })
        .into_owned();
    out.chars().take(MAX_MESSAGE_CHARS).collect()
}

fn valid_shape_key(key: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]{0,40}$").expect("valid regex")).is_match(key)
}

/// Structure of a value (types and key names), never its content.
pub fn shape(v: &Value, depth: usize) -> Value {
    match v {
        Value::Null => json!("null"),
        Value::Bool(_) => json!("bool"),
        Value::Number(_) => json!("number"),
        Value::String(_) => json!("string"),
        Value::Array(items) => {
            if depth >= MAX_SHAPE_DEPTH {
                json!("array")
            } else {
                Value::Array(items.first().map(|first| shape(first, depth + 1)).into_iter().collect())
            }
        }
        Value::Object(map) => {
            if depth >= MAX_SHAPE_DEPTH {
                return json!("object");
            }
            let mut out = serde_json::Map::new();
            for (key, value) in map {
                let name = if valid_shape_key(key) { key.as_str() } else { "[clave]" };
                out.insert(name.to_string(), shape(value, depth + 1));
            }
            Value::Object(out)
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: &'static str,
    mode: &'static str,
    os: &'static str,
}

#[derive(Serialize)]
struct StepError {
    code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<u16>,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Step {
    name: &'static str,
    ms: u64,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pages: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failed_keys: Option<usize>,
    #[serde(skip_serializing_if = "is_false")]
    skipped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<StepError>,
    #[serde(skip)]
    raw: Option<AppError>,
}

fn is_false(v: &bool) -> bool {
    !*v
}

impl Step {
    fn new(name: &'static str, started: Instant, raw: Option<AppError>) -> Self {
        Step {
            name,
            ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            ok: raw.is_none(),
            count: None,
            pages: None,
            failed_keys: None,
            skipped: false,
            error: None,
            raw,
        }
    }

    fn skipped(name: &'static str) -> Self {
        Step { skipped: true, ..Step::new(name, Instant::now(), None) }
    }
}

/// Runs a step's result through the bookkeeping: a rejected token cuts the walk, other errors are recorded.
fn record<T>(name: &'static str, started: Instant, r: AppResult<T>) -> AppResult<(Step, Option<T>)> {
    match r {
        Ok(v) => Ok((Step::new(name, started, None), Some(v))),
        Err(AppError::Unauthorized) => Err(AppError::Unauthorized),
        Err(e) => Ok((Step::new(name, started, Some(e)), None)),
    }
}

#[derive(Serialize)]
struct IssuesInfo {
    total: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorklogsInfo {
    issues_with_hours: usize,
    own_worklogs: usize,
    max_per_issue: usize,
}

#[derive(Serialize)]
struct FieldStats {
    present: usize,
    #[serde(rename = "null")]
    null_count: usize,
    missing: usize,
    shape: Value,
}

/// Field stats in the order of `jira::FIELDS`, serialized as a JSON object.
struct Fields(Vec<(&'static str, FieldStats)>);

impl Serialize for Fields {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_map(self.0.iter().map(|(k, v)| (k, v)))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LastError {
    step: &'static str,
    code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<u16>,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    app: AppInfo,
    generated_at: String,
    steps: Vec<Step>,
    issues: IssuesInfo,
    worklogs: WorklogsInfo,
    pr_mode: &'static str,
    fields: Fields,
    last_error: Option<LastError>,
}

fn field_stats(issues: &[Value]) -> Fields {
    let mut out = Vec::new();
    for &name in jira::FIELDS {
        let (mut present, mut null_count, mut missing) = (0, 0, 0);
        let mut found: Option<Value> = None;
        let mut empty_array: Option<Value> = None;
        for issue in issues {
            match issue.get("fields").and_then(|f| f.get(name)) {
                None => missing += 1,
                Some(Value::Null) => null_count += 1,
                Some(v) => {
                    present += 1;
                    if found.is_none() {
                        let s = shape(v, 0);
                        if s == json!([]) {
                            empty_array.get_or_insert(s);
                        } else {
                            found = Some(s);
                        }
                    }
                }
            }
        }
        let shape = found.or(empty_array).unwrap_or_else(|| json!("null"));
        out.push((name, FieldStats { present, null_count, missing, shape }));
    }
    Fields(out)
}

/// Walks the load against `source`. `Err` only if the token is rejected.
pub async fn build(source: &Source, facts: &SessionFacts, simulated: bool, today: NaiveDate) -> AppResult<Report> {
    let mut steps: Vec<Step> = Vec::new();

    let started = Instant::now();
    let (mut step, _) = record("myself", started, source.myself().await)?;
    step.count = Some(1);
    steps.push(step);

    let started = Instant::now();
    let mut pages = 0;
    let searched = async {
        let mut all: Vec<Value> = Vec::new();
        let mut token: Option<String> = None;
        for _ in 0..MAX_PAGES {
            pages += 1;
            let page = source.search(jira::ISSUES_JQL, jira::FIELDS, token.as_deref(), 100).await?;
            all.extend(page.issues);
            token = page.next;
            if token.is_none() {
                break;
            }
        }
        Ok(all)
    }
    .await;
    let (mut step, issues) = record("search_issues", started, searched)?;
    let issues = issues.unwrap_or_default();
    step.pages = Some(pages);
    if step.ok {
        step.count = Some(issues.len());
    }
    steps.push(step);

    let (since, after_ms) = jira::summary_window(today)?;
    let started = Instant::now();
    let (mut step, keys) = record("keys_with_hours", started, jira::keys_with_hours(source, &since).await)?;
    let keys = keys.unwrap_or_default();
    if step.ok {
        step.count = Some(keys.len());
    }
    steps.push(step);

    let mut own_worklogs = 0;
    let mut max_per_issue = 0;
    if keys.is_empty() {
        steps.push(Step::skipped("read_worklogs"));
    } else {
        let started = Instant::now();
        let read = jira::read_worklogs(source, &keys, after_ms, Some(&facts.account_id)).await;
        let (mut step, result) = record("read_worklogs", started, read)?;
        if let Some(r) = result {
            own_worklogs = r.by_key.values().map(Vec::len).sum();
            max_per_issue = r.by_key.values().map(Vec::len).max().unwrap_or(0);
            step.count = Some(own_worklogs);
            step.failed_keys = Some(r.failed.len());
        }
        steps.push(step);
    }

    let started = Instant::now();
    let (step_pr, pr_keys) = record("keys_with_pr", started, jira::keys_with_pr(source).await)?;
    let pr_mode = if step_pr.ok { "pr" } else { "keywords" };
    let pr_keys = pr_keys.unwrap_or_default();
    let mut step_pr = step_pr;
    if step_pr.ok {
        step_pr.count = Some(pr_keys.len());
    }

    let step_docs = if pr_keys.is_empty() {
        Step::skipped("find_documents")
    } else {
        let started = Instant::now();
        let (mut step, docs) = record("find_documents", started, jira::find_documents(source, &pr_keys).await)?;
        if let Some(d) = docs {
            step.count = Some(d.len());
        }
        step
    };
    // Report order: keys_with_pr comes before find_documents.
    steps.push(step_pr);
    steps.push(step_docs);

    let known = Known::new(facts, &issues);
    let mut last_error = None;
    for step in &mut steps {
        if let Some(e) = step.raw.take() {
            let status = match &e {
                AppError::Jira { status, .. } => Some(*status),
                _ => None,
            };
            let message = anonymize(&e.to_string(), &known);
            last_error = Some(LastError { step: step.name, code: e.code(), status, message: message.clone() });
            step.error = Some(StepError { code: e.code(), status, message });
        }
    }

    Ok(Report {
        app: AppInfo {
            version: env!("CARGO_PKG_VERSION"),
            mode: if simulated { "simulated" } else { "real" },
            os: std::env::consts::OS,
        },
        generated_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        steps,
        issues: IssuesInfo { total: issues.len() },
        worklogs: WorklogsInfo { issues_with_hours: keys.len(), own_worklogs, max_per_issue },
        pr_mode,
        fields: field_stats(&issues),
        last_error,
    })
}

pub fn to_text(report: &Report) -> String {
    serde_json::to_string_pretty(report).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{Credentials, Site};
    use crate::jira::http::JiraHttp;
    use crate::jira::simulated::SimulatedJira;
    use secrecy::SecretString;
    use url::Url;
    use wiremock::matchers::{body_partial_json, method, path, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const STEP_NAMES: [&str; 6] =
        ["myself", "search_issues", "keys_with_hours", "read_worklogs", "keys_with_pr", "find_documents"];

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 13).unwrap()
    }

    fn sim_facts() -> SessionFacts {
        SessionFacts {
            site: "https://ejemplo.atlassian.net".into(),
            email: "prueba@ejemplo.com".into(),
            name: "Persona de Prueba".into(),
            account_id: "sim-0001".into(),
        }
    }

    fn empty_known() -> Known {
        Known::new(&SessionFacts { site: String::new(), email: String::new(), name: String::new(), account_id: String::new() }, &[])
    }

    #[tokio::test]
    async fn simulated_report_has_no_source_strings() {
        let source = Source::Simulated(SimulatedJira::new("prueba@ejemplo.com", true, today()));
        let report = build(&source, &sim_facts(), true, today()).await.unwrap();
        let text = to_text(&report);

        let mut sensitive: Vec<String> = vec![
            "ejemplo".into(),
            "atlassian".into(),
            "Persona de Prueba".into(),
            "prueba@ejemplo.com".into(),
            "sim-0001".into(),
        ];
        let mut token = None;
        let mut keys = Vec::new();
        loop {
            let page = source.search(jira::ISSUES_JQL, jira::FIELDS, token.as_deref(), 100).await.unwrap();
            for issue in &page.issues {
                keys.push(issue["key"].as_str().unwrap().to_string());
                sensitive.push(issue["key"].as_str().unwrap().to_string());
                if let Some(s) = issue.pointer("/fields/summary").and_then(Value::as_str) {
                    sensitive.push(s.to_string());
                }
            }
            token = page.next;
            if token.is_none() {
                break;
            }
        }
        for url in jira::find_documents(&source, &keys).await.unwrap().values() {
            sensitive.push(url.clone());
        }
        assert!(sensitive.len() > 20);
        for s in &sensitive {
            assert!(!text.contains(s.as_str()), "the report leaks {s:?}");
        }

        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["app"]["mode"], "simulated");
        let names: Vec<&str> = v["steps"].as_array().unwrap().iter().map(|s| s["name"].as_str().unwrap()).collect();
        assert_eq!(names, STEP_NAMES);
        assert!(v["issues"]["total"].as_u64().unwrap() > 10);
        assert_eq!(v["prMode"], "pr");
        assert!(v["lastError"].is_null());
        let fields = v["fields"].as_object().unwrap();
        assert_eq!(fields.len(), jira::FIELDS.len());
        for name in jira::FIELDS {
            assert!(fields.contains_key(*name), "{name}");
        }
        assert!(text.find("\"summary\"").unwrap() < text.find("\"parent\"").unwrap(), "FIELDS order");
    }

    #[tokio::test]
    async fn invalid_token_is_unauthorized() {
        let source = Source::Simulated(SimulatedJira::new("prueba@ejemplo.com", false, today()));
        let r = build(&source, &sim_facts(), true, today()).await;
        assert!(matches!(r, Err(AppError::Unauthorized)));
    }

    #[tokio::test]
    async fn http_report_hides_the_error_message_data() {
        let server = MockServer::start().await;
        Mock::given(path("/rest/api/3/myself"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "accountId": "acc-SECRETO-9",
                "displayName": "Persona Confidencial",
                "emailAddress": "persona.real@empresa-ficticia.ec"
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/rest/api/3/search/jql"))
            .and(body_partial_json(json!({ "jql": jira::ISSUES_JQL })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "isLast": true,
                "issues": [
                    { "key": "SECRETO-123", "fields": {
                        "summary": "Titulo confidencial de prueba",
                        "customfield_10020": [{ "id": 7, "name": "Sprint reservado", "state": "active" }],
                        "customfield_10057": null
                    } },
                    { "key": "OCULTO-7", "fields": { "summary": "Otro titulo reservado", "customfield_10020": [] } }
                ]
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/rest/api/3/search/jql"))
            .and(body_partial_json(json!({ "jql": jira::PR_JQL })))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "errorMessages": ["No se pudo procesar SECRETO-123 de persona.real@empresa-ficticia.ec en https://empresa-ficticia.atlassian.net/browse/SECRETO-123 'Titulo confidencial de prueba' campo 'customfield_10057'"],
                "errors": {}
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/rest/api/3/search/jql"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "isLast": true,
                "issues": [{ "key": "SECRETO-123", "fields": {} }, { "key": "OCULTO-7", "fields": {} }]
            })))
            .mount(&server)
            .await;
        Mock::given(path_regex(r"^/rest/api/3/issue/[A-Z0-9_-]+/worklog$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total": 1,
                "worklogs": [{
                    "started": "2026-11-10T08:30:00.000-0500",
                    "timeSpentSeconds": 3600,
                    "author": { "accountId": "acc-SECRETO-9" }
                }]
            })))
            .mount(&server)
            .await;
        Mock::given(path("/wiki/rest/api/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "results": [] })))
            .mount(&server)
            .await;

        let credentials = Credentials::classic_token(
            Site::parse("otro-sitio.atlassian.net").unwrap(),
            "persona.real@empresa-ficticia.ec",
            SecretString::from("tok"),
        )
        .unwrap();
        let source = Source::Http(JiraHttp::with_base(credentials, Url::parse(&format!("{}/", server.uri())).unwrap()));
        let facts = SessionFacts {
            site: "https://otro-sitio.atlassian.net".into(),
            email: "persona.real@empresa-ficticia.ec".into(),
            name: "Persona Confidencial".into(),
            account_id: "acc-SECRETO-9".into(),
        };
        let text = to_text(&build(&source, &facts, false, today()).await.unwrap());

        for secret in [
            "SECRETO-123", "OCULTO-7", "Titulo confidencial", "Otro titulo", "Persona Confidencial", "persona.real",
            "empresa-ficticia", "acc-SECRETO", "atlassian.net", "127.0.0.1", "http", "Sprint reservado", "otro-sitio",
        ] {
            assert!(!text.contains(secret), "the report leaks {secret:?}:\n{text}");
        }
        for expected in [
            "ISSUE-1", "[correo]", "[url]", "customfield_10057", "\"prMode\": \"keywords\"", "\"status\": 400",
            "\"mode\": \"real\"", "\"ownWorklogs\": 2",
        ] {
            assert!(text.contains(expected), "missing {expected:?}:\n{text}");
        }
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["lastError"]["step"], "keys_with_pr");
        assert_eq!(v["lastError"]["code"], "jira");
        assert_eq!(v["steps"][4]["ok"], false);
        assert_eq!(v["steps"][5]["skipped"], true);
        assert_eq!(v["fields"]["customfield_10020"]["shape"], json!([{ "id": "number", "name": "string", "state": "string" }]));
        assert_eq!(v["fields"]["customfield_10057"]["null"], 1);
        assert_eq!(v["fields"]["customfield_10057"]["missing"], 1);
        assert_eq!(v["fields"]["customfield_10057"]["shape"], "null");
    }

    #[test]
    fn anonymize_numbers_issue_keys_consistently() {
        let known = empty_known();
        let a = anonymize("falla APP-1 y luego OPE-22; otra vez APP-1", &known);
        assert_eq!(a, "falla ISSUE-1 y luego ISSUE-2; otra vez ISSUE-1");
        assert_eq!(anonymize("OPE-22", &known), "ISSUE-2");
    }

    #[test]
    fn anonymize_replaces_known_values_urls_and_emails() {
        let facts = SessionFacts {
            site: "https://acme.atlassian.net".into(),
            email: "yo@acme.com".into(),
            name: "Ana Perez".into(),
            account_id: "712020:abc".into(),
        };
        let issues = vec![json!({ "key": "A-1", "fields": { "summary": "Cambiar tarifa" } })];
        let known = Known::new(&facts, &issues);
        let out = anonymize(
            "acme.atlassian.net Ana Perez 712020:abc yo@acme.com Cambiar tarifa https://x.io/a?b=1 otro@x.org",
            &known,
        );
        assert_eq!(out, "[sitio] [persona] [cuenta] [correo] [título] [url] [correo]");
    }

    #[test]
    fn anonymize_quotes_and_truncation() {
        let known = empty_known();
        assert_eq!(
            anonymize("campo 'customfield_10057' valor 'secreto' y \"summary\" y «otro»", &known),
            "campo 'customfield_10057' valor '[texto]' y \"summary\" y «[texto]»"
        );
        assert_eq!(anonymize("clave 'APP-5'", &known), "clave 'ISSUE-1'");
        let long = "x".repeat(500);
        assert_eq!(anonymize(&long, &known).chars().count(), 300);
    }

    #[test]
    fn shape_hides_values_and_odd_keys() {
        let v = json!({
            "b": 1, "a": "texto", "ok": true, "n": null, "list": [{ "x": 1 }, { "y": 2 }], "empty": [],
            "ISSUE KEY-1": 1, "48x48": "u", "toolongkey_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa": 1
        });
        let s = shape(&v, 0);
        assert_eq!(s["b"], "number");
        assert_eq!(s["a"], "string");
        assert_eq!(s["ok"], "bool");
        assert_eq!(s["n"], "null");
        assert_eq!(s["list"], json!([{ "x": "number" }]));
        assert_eq!(s["empty"], json!([]));
        assert!(s["[clave]"].is_string() || s["[clave]"].is_number());
        let text = s.to_string();
        assert!(!text.contains("ISSUE KEY") && !text.contains("48x48") && !text.contains("toolong"));
        assert!(text.contains("[clave]"));
    }

    #[test]
    fn shape_limits_depth() {
        let deep = json!({ "a": { "b": { "c": { "d": { "e": 1 } } } }, "l": [[[[[1]]]]] });
        let s = shape(&deep, 0);
        assert_eq!(s["a"]["b"]["c"]["d"], "object");
        assert_eq!(s["l"][0][0][0], "array");
    }
}
