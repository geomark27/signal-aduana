//! Access to Jira and Confluence: real source (HTTP) or simulated, and high-level operations.
//!
//! Tauri commands always work with `Source`; the choice between the real API and the
//! fictitious data is made once, at sign-in.

pub mod http;
pub mod model;
pub mod simulated;

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use futures::stream::{self, StreamExt};
use regex::Regex;
use serde::Serialize;
use serde_json::Value;

use crate::error::{AppError, AppResult};
use model::{Issue, User, Worklog};

/// A page of results from `POST /rest/api/3/search/jql`.
#[derive(Debug)]
pub struct SearchPage {
    pub issues: Vec<Value>,
    pub next: Option<String>,
}

/// A Confluence page found by CQL (absolute `url`).
pub struct WikiPage {
    pub title: String,
    pub url: String,
}

/// A remote link of a Jira issue.
pub struct RemoteLink {
    pub title: String,
    pub url: String,
}

pub const ISSUES_JQL: &str = "assignee = currentUser() OR worklogAuthor = currentUser() ORDER BY updated DESC";
pub const PR_JQL: &str =
    "(assignee = currentUser() OR worklogAuthor = currentUser()) AND development[pullrequests].all > 0";
pub const FIELDS: &[&str] = &[
    "summary",
    "status",
    "statuscategorychangedate",
    "project",
    "issuetype",
    "priority",
    "created",
    "updated",
    "resolutiondate",
    "duedate",
    "timeoriginalestimate",
    "timespent",
    "worklog",
    "components",
    "labels",
    "customfield_10031",
    "customfield_10020",
    "customfield_10015",
    "customfield_10057",
    "parent",
];

/// A valid Jira key: `APP-123`. Keeps arbitrary text from reaching a URL or a JQL.
pub fn validate_key(key: &str) -> AppResult<&str> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^[A-Z][A-Z0-9_]{0,19}-[0-9]{1,9}$").expect("regex válida"));
    if re.is_match(key) {
        Ok(key)
    } else {
        Err(AppError::InvalidKey(key.chars().take(40).collect()))
    }
}

/// Enum dispatch (no async-trait): each method does a `match` and delegates.
pub enum Source {
    Http(http::JiraHttp),
    Simulated(simulated::SimulatedJira),
}

impl Source {
    pub async fn myself(&self) -> AppResult<User> {
        match self {
            Source::Http(h) => h.myself().await,
            Source::Simulated(s) => s.myself().await,
        }
    }

    pub async fn search(
        &self,
        jql: &str,
        fields: &[&str],
        next: Option<&str>,
        size: u32,
    ) -> AppResult<SearchPage> {
        match self {
            Source::Http(h) => h.search(jql, fields, next, size).await,
            Source::Simulated(s) => s.search(jql, fields, next, size).await,
        }
    }

    /// The issue's worklogs (raw Jira JSON), from any author.
    pub async fn worklogs(&self, key: &str, since_ms: i64) -> AppResult<Vec<Value>> {
        match self {
            Source::Http(h) => h.worklogs(key, since_ms).await,
            Source::Simulated(s) => s.worklogs(key, since_ms).await,
        }
    }

    pub async fn confluence(&self, cql: &str) -> AppResult<Vec<WikiPage>> {
        match self {
            Source::Http(h) => h.confluence(cql).await,
            Source::Simulated(s) => s.confluence(cql).await,
        }
    }

    pub async fn links(&self, key: &str) -> AppResult<Vec<RemoteLink>> {
        match self {
            Source::Http(h) => h.links(key).await,
            Source::Simulated(s) => s.links(key).await,
        }
    }
}

/// Walks the pages of a search (with a cap) and returns all the `issues`.
async fn search_all(f: &Source, jql: &str, fields: &[&str], size: u32, max_pages: usize) -> AppResult<Vec<Value>> {
    let mut all = Vec::new();
    let mut token: Option<String> = None;
    for _ in 0..max_pages {
        let page = f.search(jql, fields, token.as_deref(), size).await?;
        all.extend(page.issues);
        token = page.next;
        if token.is_none() {
            break;
        }
    }
    Ok(all)
}

fn keys_of(issues: &[Value]) -> Vec<String> {
    issues.iter().filter_map(|v| v.get("key").and_then(Value::as_str).map(String::from)).collect()
}

/// The user's issues (assigned or with their worklogs), already converted to the UI model.
pub async fn load_issues(f: &Source, me: Option<&str>) -> AppResult<Vec<Issue>> {
    let issues = search_all(f, ISSUES_JQL, FIELDS, 100, 20).await?;
    Ok(issues.iter().filter_map(|v| Issue::from_jira(v, me)).collect())
}

/// Keys of the issues where the user logged hours since `since` (`YYYY-MM-DD`).
pub async fn keys_with_hours(f: &Source, since: &str) -> AppResult<Vec<String>> {
    let date = chrono::NaiveDate::parse_from_str(since, "%Y-%m-%d")
        .ok()
        .filter(|_| since.len() == 10)
        .ok_or_else(|| AppError::Format(format!("fecha no válida (se esperaba AAAA-MM-DD): {}", since.chars().take(20).collect::<String>())))?;
    let jql = format!("worklogAuthor = currentUser() AND worklogDate >= \"{}\"", date.format("%Y-%m-%d"));
    Ok(keys_of(&search_all(f, &jql, &["summary"], 100, 10).await?))
}

/// Keys of the user's issues that have pull requests in the development panel.
pub async fn keys_with_pr(f: &Source) -> AppResult<Vec<String>> {
    Ok(keys_of(&search_all(f, PR_JQL, &["summary"], 100, 20).await?))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorklogsResult {
    pub by_key: HashMap<String, Vec<Worklog>>,
    pub failed: Vec<String>,
    pub error: Option<String>,
}

/// Reads the user's own worklogs for each issue (4 queries at a time).
///
/// An issue that fails is reported in `failed`; if it fails because of credentials, or all fail, it is an error.
pub async fn read_worklogs(f: &Source, keys: &[String], since_ms: i64, me: Option<&str>) -> AppResult<WorklogsResult> {
    for c in keys {
        validate_key(c)?;
    }
    let mut unique: Vec<String> = Vec::new();
    for c in keys {
        if !unique.contains(c) {
            unique.push(c.clone());
        }
    }
    let results: Vec<(String, AppResult<Vec<Value>>)> = stream::iter(unique.iter().cloned())
        .map(|c| async move {
            let r = f.worklogs(&c, since_ms).await;
            (c, r)
        })
        .buffer_unordered(4)
        .collect()
        .await;

    let mut by_key = HashMap::new();
    let mut failed = Vec::new();
    let mut last: Option<AppError> = None;
    for (key, r) in results {
        match r {
            Ok(values) => {
                let worklogs: Vec<Worklog> =
                    values.iter().filter_map(Worklog::from_jira).filter(|r| r.is_by(me)).collect();
                by_key.insert(key, worklogs);
            }
            Err(AppError::Unauthorized) => return Err(AppError::Unauthorized),
            Err(e) => {
                failed.push(key);
                last = Some(e);
            }
        }
    }
    failed.sort();
    if !unique.is_empty() && failed.len() == unique.len() {
        if let Some(e) = last {
            return Err(e);
        }
    }
    Ok(WorklogsResult { by_key, failed, error: last.map(|e| e.to_string()) })
}

/// What the UI does on load (no documents or PRs): issues + own worklogs since the Monday of
/// `today - 98 days` (instant 00:00 of that Monday in UTC-5). Each issue ends up with `worklogs`, `wlTotal` and,
/// if `spent` was 0, `spent` = sum of its worklogs. Returns the input format of `compute_view`.
pub async fn load_for_summary(
    f: &Source,
    me: Option<&str>,
    today: chrono::NaiveDate,
) -> AppResult<Vec<serde_json::Map<String, Value>>> {
    use chrono::{Datelike, Duration};

    let issues = load_issues(f, me).await?;
    let mut issues: Vec<serde_json::Map<String, Value>> = issues
        .iter()
        .map(|t| match serde_json::to_value(t) {
            Ok(Value::Object(m)) => Ok(m),
            _ => Err(AppError::Format("tarea no serializable".into())),
        })
        .collect::<AppResult<_>>()?;

    let back = today - Duration::days(98);
    let since_monday = back - Duration::days(i64::from(back.weekday().num_days_from_monday()));
    let since = since_monday.format("%Y-%m-%d").to_string();
    let after_ms = since_monday
        .and_hms_opt(5, 0, 0)
        .ok_or_else(|| AppError::Format("hora no válida".into()))?
        .and_utc()
        .timestamp_millis();

    let keys = keys_with_hours(f, &since).await?;
    let hours = if keys.is_empty() { None } else { Some(read_worklogs(f, &keys, after_ms, me).await?) };

    for t in &mut issues {
        let key = t.get("key").and_then(Value::as_str).unwrap_or_default().to_string();
        let wl = match hours.as_ref().and_then(|h| h.by_key.get(&key)) {
            Some(r) => serde_json::to_value(r).map_err(|e| AppError::Format(e.to_string()))?,
            None => Value::Array(Vec::new()),
        };
        let sum: i64 = wl.as_array().map(|a| a.iter().filter_map(|w| w["s"].as_i64()).sum()).unwrap_or(0);
        t.insert("wlTotal".into(), Value::from(wl.as_array().map_or(0, Vec::len)));
        if t.get("spent").and_then(Value::as_i64).unwrap_or(0) == 0 {
            t.insert("spent".into(), Value::from(sum));
        }
        t.insert("worklogs".into(), wl);
    }
    Ok(issues)
}

/// Looks for each issue's «Documento de Despliegue»: first in Confluence (by title) and, for those
/// that do not show up, in the issue's remote links. Returns `key → url`.
pub async fn find_documents(f: &Source, keys: &[String]) -> AppResult<HashMap<String, String>> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(?i)Despliegue\s*-?\s*([A-Z][A-Z0-9]+-\d+)").expect("regex válida"));

    for c in keys {
        validate_key(c)?;
    }
    let mut unique: Vec<&str> = Vec::new();
    for c in keys {
        if !unique.contains(&c.as_str()) {
            unique.push(c);
        }
    }

    let mut docs: HashMap<String, String> = HashMap::new();
    for group in unique.chunks(25) {
        let filters: Vec<String> = group.iter().map(|c| format!("title ~ \"\\\"{c}\\\"\"")).collect();
        let cql = format!("type = page AND title ~ \"Despliegue\" AND ({})", filters.join(" OR "));
        for page in f.confluence(&cql).await? {
            if let Some(m) = re.captures(&page.title) {
                let key = m[1].to_uppercase();
                if unique.contains(&key.as_str()) {
                    docs.entry(key).or_insert(page.url);
                }
            }
        }
    }

    // Fallback: Confluence search is slow to index new pages, but the issue's remote link
    // already exists. Errors in this step are ignored (it stays as «no document»).
    let missing: Vec<String> =
        unique.iter().copied().filter(|c| !docs.contains_key(*c)).take(60).map(String::from).collect();
    let found: Vec<Option<(String, String)>> = stream::iter(missing)
        .map(|c| async move {
            let links = f.links(&c).await.ok()?;
            let best = links
                .into_iter()
                .filter(|e| e.title.to_lowercase().contains("despliegue") || e.url.contains("Documento+de+Despliegue"))
                .min_by_key(|e| !e.url.contains("/spaces/"))?;
            Some((c, best.url))
        })
        .buffer_unordered(5)
        .collect()
        .await;
    docs.extend(found.into_iter().flatten());
    // Only requested keys (by construction), without duplicates.
    let requested: HashSet<&str> = unique.iter().copied().collect();
    docs.retain(|k, _| requested.contains(k.as_str()));
    Ok(docs)
}

#[cfg(test)]
mod tests {
    use super::simulated::SimulatedJira;
    use super::*;
    use chrono::NaiveDate;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
    }

    fn sim(valid: bool) -> Source {
        Source::Simulated(SimulatedJira::new("prueba@ejemplo.com", valid, today()))
    }

    #[test]
    fn validates_keys() {
        assert_eq!(validate_key("APP-123").unwrap(), "APP-123");
        assert!(validate_key("A1_B-9").is_ok());
        for bad in ["app-1", "APP-", "-1", "APP-1x", "../../myself", "APP-1\n", "", "1APP-2", "APP-1234567890"] {
            assert!(matches!(validate_key(bad), Err(AppError::InvalidKey(_))), "{bad:?}");
        }
    }

    #[tokio::test]
    async fn load_issues_paginates_without_duplicates() {
        let f = sim(true);
        let issues = load_issues(&f, Some("sim-0001")).await.unwrap();
        let keys: HashSet<&str> = issues.iter().map(|t| t.key.as_str()).collect();
        assert_eq!(keys.len(), issues.len(), "hay claves repetidas");
        assert!(issues.len() > 10, "debe ocupar más de una página de 10");
        // The simulated source serves pages of 10: confirms that pagination really happened.
        let first = f.search(ISSUES_JQL, FIELDS, None, 100).await.unwrap();
        assert_eq!(first.issues.len(), 10);
        assert!(first.next.is_some());
        // No foreign worklog sneaks into the issues.
        assert!(issues.iter().flat_map(|t| &t.worklogs).all(|w| w.a.as_deref() == Some("sim-0001")));
    }

    #[tokio::test]
    async fn read_worklogs_filters_other_author() {
        let f = sim(true);
        let keys = vec!["APP-401".to_string(), "AFO-101".to_string()];
        let raw = f.worklogs("APP-401", 0).await.unwrap();
        assert!(raw.iter().any(|w| w["author"]["accountId"] == "sim-0002"), "el simulado debe traer horas ajenas");
        let r = read_worklogs(&f, &keys, 0, Some("sim-0001")).await.unwrap();
        assert!(r.failed.is_empty() && r.error.is_none());
        let app = &r.by_key["APP-401"];
        assert!(!app.is_empty());
        assert!(app.len() < raw.len());
        assert!(app.iter().all(|x| x.a.as_deref() == Some("sim-0001")));
        let js = serde_json::to_value(&r).unwrap();
        assert!(js.get("byKey").is_some());
    }

    #[tokio::test]
    async fn read_worklogs_with_invalid_token_is_error() {
        let f = sim(false);
        let r = read_worklogs(&f, &["APP-401".to_string()], 0, None).await;
        assert!(matches!(r, Err(AppError::Unauthorized)));
        assert!(matches!(f.myself().await, Err(AppError::Unauthorized)));
    }

    #[tokio::test]
    async fn read_worklogs_rejects_invalid_keys() {
        let f = sim(true);
        let r = read_worklogs(&f, &["APP-401".to_string(), "x y".to_string()], 0, None).await;
        assert!(matches!(r, Err(AppError::InvalidKey(_))));
    }

    #[tokio::test]
    async fn find_documents_via_confluence_and_via_link() {
        let f = sim(true);
        let with_pr = keys_with_pr(&f).await.unwrap();
        let docs = find_documents(&f, &with_pr).await.unwrap();
        // Via Confluence.
        assert!(docs["AFO-102"].contains("/wiki/spaces/"));
        // Only as a remote link (the fallback).
        assert!(docs["APP-402"].contains("Documento+de+Despliegue"));
        // No document.
        assert!(!docs.contains_key("APP-403"));
        // Only requested keys.
        assert!(docs.keys().all(|k| with_pr.contains(k)));
        let partial = find_documents(&f, &["AFO-102".to_string()]).await.unwrap();
        assert_eq!(partial.len(), 1);
    }

    #[tokio::test]
    async fn keys_with_pr_only_returns_issues_with_pr() {
        let f = sim(true);
        let with_pr = keys_with_pr(&f).await.unwrap();
        assert!(with_pr.contains(&"APP-403".to_string()));
        assert!(!with_pr.contains(&"OPE-701".to_string()), "la reunión no tiene PR");
    }

    #[tokio::test]
    async fn keys_with_hours_validates_the_date() {
        let f = sim(true);
        for bad in ["2026-13-01", "ayer", "2026-1-5", "", "2026-11-16'; DROP", "16/11/2026"] {
            assert!(matches!(keys_with_hours(&f, bad).await, Err(AppError::Format(_))), "{bad:?}");
        }
        let all = keys_with_hours(&f, "2020-01-01").await.unwrap();
        let recent = keys_with_hours(&f, "2026-11-10").await.unwrap();
        assert!(all.contains(&"OPE-701".to_string()));
        assert!(!recent.is_empty() && recent.len() < all.len());
        assert!(!all.contains(&"ADN-301".to_string()), "ADN-301 no tiene horas");
    }

    fn test_date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 13).unwrap()
    }

    fn sim_friday(valid: bool) -> Source {
        Source::Simulated(SimulatedJira::new("a@b.com", valid, test_date()))
    }

    #[tokio::test]
    async fn load_for_summary_returns_issues_with_own_worklogs() {
        use chrono::{Datelike, Duration};
        let f = sim_friday(true);
        let issues = load_for_summary(&f, Some("sim-0001"), test_date()).await.unwrap();
        assert!(!issues.is_empty());
        let back = test_date() - Duration::days(98);
        let since = (back - Duration::days(i64::from(back.weekday().num_days_from_monday()))).format("%Y-%m-%d").to_string();
        let mut had_worklogs = false;
        for t in &issues {
            assert!(t.get("key").and_then(Value::as_str).is_some());
            assert!(!t.contains_key("hasPR"));
            let wl = t.get("worklogs").and_then(Value::as_array).expect("worklogs es un arreglo");
            for w in wl {
                had_worklogs = true;
                assert_ne!(w["a"].as_str(), Some("sim-0002"), "registro ajeno");
                assert!(w["d"].as_str().unwrap() >= since.as_str(), "registro anterior al lunes de hoy - 98 días");
            }
        }
        assert!(had_worklogs);
    }

    #[tokio::test]
    async fn load_for_summary_with_invalid_token_is_unauthorized() {
        let f = sim_friday(false);
        let r = load_for_summary(&f, None, test_date()).await;
        assert!(matches!(r, Err(AppError::Unauthorized)));
    }

    #[tokio::test]
    async fn load_for_summary_fills_spent_with_worklog_sum() {
        let f = sim_friday(true);
        let originals = load_issues(&f, Some("sim-0001")).await.unwrap();
        let without_spent: Vec<&str> = originals.iter().filter(|t| t.spent == 0).map(|t| t.key.as_str()).collect();
        assert!(!without_spent.is_empty(), "el simulado debe traer alguna tarea sin tiempo gastado");
        let issues = load_for_summary(&f, Some("sim-0001"), test_date()).await.unwrap();
        for t in issues.iter().filter(|t| without_spent.contains(&t["key"].as_str().unwrap())) {
            let sum: i64 = t["worklogs"].as_array().unwrap().iter().filter_map(|w| w["s"].as_i64()).sum();
            assert_eq!(t["spent"].as_i64(), Some(sum), "{}", t["key"]);
        }
    }
}
