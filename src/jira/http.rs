//! Real client of the Jira Cloud and Confluence REST API.
//!
//! | Use                     | Endpoint                                   |
//! |-------------------------|--------------------------------------------|
//! | Session user            | `GET  /rest/api/3/myself`                  |
//! | JQL search              | `POST /rest/api/3/search/jql`              |
//! | Issue worklogs          | `GET  /rest/api/3/issue/{key}/worklog`     |
//! | Issue links             | `GET  /rest/api/3/issue/{key}/remotelink`  |
//! | Confluence search       | `GET  /wiki/rest/api/search?cql=…`         |

use std::time::Duration;

use reqwest::header::{ACCEPT, RETRY_AFTER};
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use url::Url;

use super::{validate_key, RemoteLink, SearchPage, WikiPage};
use crate::auth::{Autenticador, Credenciales};
use crate::error::{ErrorApp, Resultado};
use crate::jira::model::User;

const TIMEOUT: Duration = Duration::from_secs(30);
const MAX_429_WAIT_SECS: u64 = 10;

pub struct JiraHttp {
    http: Client,
    credentials: Credenciales,
    base: Url,
}

impl JiraHttp {
    pub fn new(credentials: Credenciales) -> Resultado<Self> {
        let base = credentials.url_base().clone();
        let http = Self::client().https_only(true).build()?;
        Ok(JiraHttp { http, credentials, base })
    }

    /// Tests only: points to a local simulated server (http://127.0.0.1:port).
    #[cfg(test)]
    pub fn with_base(credentials: Credenciales, base: Url) -> Self {
        JiraHttp { http: Self::client().build().unwrap(), credentials, base }
    }

    fn client() -> reqwest::ClientBuilder {
        Client::builder()
            .timeout(TIMEOUT)
            .user_agent(concat!("signal-aduana/", env!("CARGO_PKG_VERSION")))
    }

    fn url(&self, path: &str) -> Resultado<Url> {
        self.base.join(path).map_err(|e| ErrorApp::Formato(e.to_string()))
    }

    /// Signs, sends and checks the response. On a 429 it waits as long as Jira says and retries once.
    async fn send(&self, req: RequestBuilder) -> Resultado<Response> {
        let req = self.credentials.firmar(req.header(ACCEPT, "application/json"));
        let copy = req.try_clone();
        let resp = req.send().await?;
        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            if let Some(copy) = copy {
                let wait = resp
                    .headers()
                    .get(RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(2)
                    .min(MAX_429_WAIT_SECS);
                tokio::time::sleep(Duration::from_secs(wait)).await;
                return check_response(copy.send().await?).await;
            }
        }
        check_response(resp).await
    }

    async fn json<T: DeserializeOwned>(&self, req: RequestBuilder) -> Resultado<T> {
        self.send(req)
            .await?
            .json::<T>()
            .await
            .map_err(|e| ErrorApp::Formato(e.without_url().to_string()))
    }

    pub async fn myself(&self) -> Resultado<User> {
        let v: Value = self.json(self.http.get(self.url("rest/api/3/myself")?)).await?;
        User::from_jira(&v).ok_or_else(|| ErrorApp::Formato("falta accountId en /myself".into()))
    }

    pub async fn search(&self, jql: &str, fields: &[&str], next: Option<&str>, size: u32) -> Resultado<SearchPage> {
        let mut body = json!({ "jql": jql, "fields": fields, "maxResults": size });
        if let Some(t) = next {
            body["nextPageToken"] = json!(t);
        }
        let v: Value = self.json(self.http.post(self.url("rest/api/3/search/jql")?).json(&body)).await?;
        let issues = v.get("issues").and_then(Value::as_array).cloned().unwrap_or_default();
        let is_last = v.get("isLast").and_then(Value::as_bool).unwrap_or(false);
        let next = if is_last {
            None
        } else {
            v.get("nextPageToken").and_then(Value::as_str).map(String::from)
        };
        Ok(SearchPage { issues, next })
    }

    /// All the issue's worklogs started after `since_ms` (from any author).
    pub async fn worklogs(&self, key: &str, since_ms: i64) -> Resultado<Vec<Value>> {
        let key = validate_key(key)?;
        let url = self.url(&format!("rest/api/3/issue/{key}/worklog"))?;
        let mut all = Vec::new();
        for _ in 0..20 {
            let req = self.http.get(url.clone()).query(&[
                ("startedAfter", since_ms.to_string()),
                ("startAt", all.len().to_string()),
                ("maxResults", "1000".to_string()),
            ]);
            let v: Value = self.json(req).await?;
            let page = v.get("worklogs").and_then(Value::as_array).cloned().unwrap_or_default();
            let total = v.get("total").and_then(Value::as_u64).unwrap_or(0) as usize;
            let is_empty = page.is_empty();
            all.extend(page);
            if is_empty || all.len() >= total {
                break;
            }
        }
        Ok(all)
    }

    pub async fn confluence(&self, cql: &str) -> Resultado<Vec<WikiPage>> {
        let req = self
            .http
            .get(self.url("wiki/rest/api/search")?)
            .query(&[("cql", cql), ("limit", "100")]);
        let v: Value = self.json(req).await?;
        let wiki_base = v
            .pointer("/_links/base")
            .and_then(Value::as_str)
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| format!("{}wiki", self.base));
        let pages = v
            .get("results")
            .and_then(Value::as_array)
            .map(|r| {
                r.iter()
                    .filter_map(|r| {
                        // `content.title` comes clean; `title` may carry highlight marks.
                        let title = r
                            .pointer("/content/title")
                            .or_else(|| r.get("title"))
                            .and_then(Value::as_str)?
                            .to_string();
                        let path = r
                            .pointer("/content/_links/webui")
                            .or_else(|| r.get("url"))
                            .and_then(Value::as_str)?;
                        let url = if path.starts_with("https://") {
                            path.to_string()
                        } else {
                            format!("{wiki_base}{path}")
                        };
                        Some(WikiPage { title, url })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(pages)
    }

    pub async fn links(&self, key: &str) -> Resultado<Vec<RemoteLink>> {
        let key = validate_key(key)?;
        let v: Value = self
            .json(self.http.get(self.url(&format!("rest/api/3/issue/{key}/remotelink"))?))
            .await?;
        Ok(v.as_array()
            .map(|a| {
                a.iter()
                    .map(|e| RemoteLink {
                        title: e.pointer("/object/title").and_then(Value::as_str).unwrap_or("").to_string(),
                        url: e.pointer("/object/url").and_then(Value::as_str).unwrap_or("").to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default())
    }
}

/// Converts HTTP status codes into application errors.
async fn check_response(resp: Response) -> Resultado<Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    Err(match status {
        StatusCode::UNAUTHORIZED => ErrorApp::NoAutorizado,
        StatusCode::FORBIDDEN => ErrorApp::SinPermiso,
        StatusCode::TOO_MANY_REQUESTS => ErrorApp::Limite,
        _ => {
            let body = resp.text().await.unwrap_or_default();
            ErrorApp::Jira { estado: status.as_u16(), detalle: summarize_error(&body) }
        }
    })
}

/// Jira returns `{"errorMessages": [...], "errors": {...}}`; otherwise the text is truncated.
fn summarize_error(body: &str) -> String {
    if let Ok(v) = serde_json::from_str::<Value>(body) {
        let mut parts: Vec<String> = v
            .get("errorMessages")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|m| m.as_str().map(String::from)).collect())
            .unwrap_or_default();
        if let Some(obj) = v.get("errors").and_then(Value::as_object) {
            parts.extend(obj.iter().map(|(k, m)| format!("{k}: {}", m.as_str().unwrap_or(""))));
        }
        if !parts.is_empty() {
            return parts.join(" · ");
        }
    }
    body.chars().take(300).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::Sitio;
    use secrecy::SecretString;
    use wiremock::matchers::{body_partial_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn client(server: &MockServer) -> JiraHttp {
        let credentials = Credenciales::token_clasico(
            Sitio::parse("prueba.atlassian.net").unwrap(),
            "a@b.com",
            SecretString::from("tok"),
        )
        .unwrap();
        JiraHttp::with_base(credentials, Url::parse(&format!("{}/", server.uri())).unwrap())
    }

    #[tokio::test]
    async fn myself_sends_basic_auth_and_reads_user() {
        let s = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/rest/api/3/myself"))
            .and(header("authorization", "Basic YUBiLmNvbTp0b2s="))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "accountId": "abc", "displayName": "Persona de Prueba", "emailAddress": "a@b.com"
            })))
            .expect(1)
            .mount(&s)
            .await;
        let u = client(&s).await.myself().await.unwrap();
        assert_eq!(u.account_id, "abc");
        assert_eq!(u.name, "Persona de Prueba");
    }

    #[tokio::test]
    async fn status_401_is_unauthorized_and_403_is_forbidden() {
        let s = MockServer::start().await;
        Mock::given(path("/rest/api/3/myself")).respond_with(ResponseTemplate::new(401)).mount(&s).await;
        Mock::given(path("/rest/api/3/issue/APP-1/remotelink")).respond_with(ResponseTemplate::new(403)).mount(&s).await;
        let c = client(&s).await;
        assert!(matches!(c.myself().await, Err(ErrorApp::NoAutorizado)));
        assert!(matches!(c.links("APP-1").await, Err(ErrorApp::SinPermiso)));
    }

    #[tokio::test]
    async fn error_400_shows_jira_message() {
        let s = MockServer::start().await;
        Mock::given(path("/rest/api/3/search/jql"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({"errorMessages": ["JQL inválido"], "errors": {}})))
            .mount(&s)
            .await;
        match client(&s).await.search("x", &["summary"], None, 100).await {
            Err(ErrorApp::Jira { estado: status, detalle: detail }) => {
                assert_eq!(status, 400);
                assert_eq!(detail, "JQL inválido");
            }
            other => panic!("esperaba error de Jira, llegó {other:?}"),
        }
    }

    #[tokio::test]
    async fn search_paginates_with_next_page_token() {
        let s = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/rest/api/3/search/jql"))
            .and(body_partial_json(json!({"jql": "q", "maxResults": 50})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "issues": [{"key": "APP-1", "fields": {}}], "nextPageToken": "T2", "isLast": false
            })))
            .mount(&s)
            .await;
        let p = client(&s).await.search("q", &["summary"], None, 50).await.unwrap();
        assert_eq!(p.issues.len(), 1);
        assert_eq!(p.next.as_deref(), Some("T2"));
    }

    #[tokio::test]
    async fn worklogs_walks_pages() {
        let s = MockServer::start().await;
        let w = |n: i64| json!({"started": "2026-11-10T08:30:00.000-0500", "timeSpentSeconds": n, "author": {"accountId": "yo"}});
        Mock::given(path("/rest/api/3/issue/APP-1/worklog"))
            .and(query_param("startAt", "0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"total": 3, "worklogs": [w(1), w(2)]})))
            .mount(&s)
            .await;
        Mock::given(path("/rest/api/3/issue/APP-1/worklog"))
            .and(query_param("startAt", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"total": 3, "worklogs": [w(3)]})))
            .mount(&s)
            .await;
        let all = client(&s).await.worklogs("APP-1", 0).await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn confluence_builds_absolute_url_and_uses_clean_title() {
        let s = MockServer::start().await;
        Mock::given(path("/wiki/rest/api/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [{
                    "title": "@@@hl@@@Documento@@@endhl@@@ de Despliegue - APP-1",
                    "content": {"title": "Documento de Despliegue - APP-1 - Ajuste", "_links": {"webui": "/spaces/DEV/pages/1"}}
                }],
                "_links": {"base": "https://prueba.atlassian.net/wiki"}
            })))
            .mount(&s)
            .await;
        let p = client(&s).await.confluence("type = page").await.unwrap();
        assert_eq!(p[0].title, "Documento de Despliegue - APP-1 - Ajuste");
        assert_eq!(p[0].url, "https://prueba.atlassian.net/wiki/spaces/DEV/pages/1");
    }

    #[tokio::test]
    async fn invalid_key_never_reaches_the_network() {
        let s = MockServer::start().await;
        let c = client(&s).await;
        assert!(matches!(c.worklogs("../../myself", 0).await, Err(ErrorApp::ClaveInvalida(_))));
        assert!(s.received_requests().await.unwrap().is_empty());
    }
}
