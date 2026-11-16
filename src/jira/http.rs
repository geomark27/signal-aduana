//! Cliente real de la API REST de Jira Cloud y Confluence.
//!
//! | Uso                     | Endpoint                                   |
//! |-------------------------|--------------------------------------------|
//! | Usuario de la sesión    | `GET  /rest/api/3/myself`                  |
//! | Búsqueda JQL            | `POST /rest/api/3/search/jql`              |
//! | Horas de una tarea      | `GET  /rest/api/3/issue/{clave}/worklog`   |
//! | Enlaces de una tarea    | `GET  /rest/api/3/issue/{clave}/remotelink`|
//! | Búsqueda en Confluence  | `GET  /wiki/rest/api/search?cql=…`         |

use std::time::Duration;

use reqwest::header::{ACCEPT, RETRY_AFTER};
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use url::Url;

use super::{validar_clave, EnlaceRemoto, PaginaBusqueda, PaginaWiki};
use crate::auth::{Autenticador, Credenciales};
use crate::error::{ErrorApp, Resultado};
use crate::jira::modelo::Usuario;

const TIEMPO_MAXIMO: Duration = Duration::from_secs(30);
const ESPERA_MAXIMA_429: u64 = 10;

pub struct JiraHttp {
    http: Client,
    cred: Credenciales,
    base: Url,
}

impl JiraHttp {
    pub fn nuevo(cred: Credenciales) -> Resultado<Self> {
        let base = cred.url_base().clone();
        let http = Self::cliente().https_only(true).build()?;
        Ok(JiraHttp { http, cred, base })
    }

    /// Solo para pruebas: apunta a un servidor local simulado (http://127.0.0.1:puerto).
    #[cfg(test)]
    pub fn con_base(cred: Credenciales, base: Url) -> Self {
        JiraHttp { http: Self::cliente().build().unwrap(), cred, base }
    }

    fn cliente() -> reqwest::ClientBuilder {
        Client::builder()
            .timeout(TIEMPO_MAXIMO)
            .user_agent(concat!("signal-aduana/", env!("CARGO_PKG_VERSION")))
    }

    fn url(&self, ruta: &str) -> Resultado<Url> {
        self.base.join(ruta).map_err(|e| ErrorApp::Formato(e.to_string()))
    }

    /// Firma, envía y revisa la respuesta. Ante un 429 espera lo que indique Jira y reintenta una vez.
    async fn enviar(&self, req: RequestBuilder) -> Resultado<Response> {
        let req = self.cred.firmar(req.header(ACCEPT, "application/json"));
        let copia = req.try_clone();
        let resp = req.send().await?;
        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            if let Some(copia) = copia {
                let espera = resp
                    .headers()
                    .get(RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(2)
                    .min(ESPERA_MAXIMA_429);
                tokio::time::sleep(Duration::from_secs(espera)).await;
                return revisar(copia.send().await?).await;
            }
        }
        revisar(resp).await
    }

    async fn json<T: DeserializeOwned>(&self, req: RequestBuilder) -> Resultado<T> {
        self.enviar(req)
            .await?
            .json::<T>()
            .await
            .map_err(|e| ErrorApp::Formato(e.without_url().to_string()))
    }

    pub async fn yo(&self) -> Resultado<Usuario> {
        let v: Value = self.json(self.http.get(self.url("rest/api/3/myself")?)).await?;
        Usuario::desde_jira(&v).ok_or_else(|| ErrorApp::Formato("falta accountId en /myself".into()))
    }

    pub async fn buscar(&self, jql: &str, campos: &[&str], siguiente: Option<&str>, tam: u32) -> Resultado<PaginaBusqueda> {
        let mut cuerpo = json!({ "jql": jql, "fields": campos, "maxResults": tam });
        if let Some(t) = siguiente {
            cuerpo["nextPageToken"] = json!(t);
        }
        let v: Value = self.json(self.http.post(self.url("rest/api/3/search/jql")?).json(&cuerpo)).await?;
        let issues = v.get("issues").and_then(Value::as_array).cloned().unwrap_or_default();
        let ultima = v.get("isLast").and_then(Value::as_bool).unwrap_or(false);
        let siguiente = if ultima {
            None
        } else {
            v.get("nextPageToken").and_then(Value::as_str).map(String::from)
        };
        Ok(PaginaBusqueda { issues, siguiente })
    }

    /// Todos los registros de horas de la tarea iniciados después de `desde_ms` (de cualquier autor).
    pub async fn worklogs(&self, clave: &str, desde_ms: i64) -> Resultado<Vec<Value>> {
        let clave = validar_clave(clave)?;
        let url = self.url(&format!("rest/api/3/issue/{clave}/worklog"))?;
        let mut todos = Vec::new();
        for _ in 0..20 {
            let req = self.http.get(url.clone()).query(&[
                ("startedAfter", desde_ms.to_string()),
                ("startAt", todos.len().to_string()),
                ("maxResults", "1000".to_string()),
            ]);
            let v: Value = self.json(req).await?;
            let pagina = v.get("worklogs").and_then(Value::as_array).cloned().unwrap_or_default();
            let total = v.get("total").and_then(Value::as_u64).unwrap_or(0) as usize;
            let vacia = pagina.is_empty();
            todos.extend(pagina);
            if vacia || todos.len() >= total {
                break;
            }
        }
        Ok(todos)
    }

    pub async fn confluence(&self, cql: &str) -> Resultado<Vec<PaginaWiki>> {
        let req = self
            .http
            .get(self.url("wiki/rest/api/search")?)
            .query(&[("cql", cql), ("limit", "100")]);
        let v: Value = self.json(req).await?;
        let base_wiki = v
            .pointer("/_links/base")
            .and_then(Value::as_str)
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| format!("{}wiki", self.base));
        let paginas = v
            .get("results")
            .and_then(Value::as_array)
            .map(|r| {
                r.iter()
                    .filter_map(|r| {
                        // `content.title` viene limpio; `title` puede traer marcas de resaltado.
                        let titulo = r
                            .pointer("/content/title")
                            .or_else(|| r.get("title"))
                            .and_then(Value::as_str)?
                            .to_string();
                        let ruta = r
                            .pointer("/content/_links/webui")
                            .or_else(|| r.get("url"))
                            .and_then(Value::as_str)?;
                        let url = if ruta.starts_with("https://") {
                            ruta.to_string()
                        } else {
                            format!("{base_wiki}{ruta}")
                        };
                        Some(PaginaWiki { titulo, url })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(paginas)
    }

    pub async fn enlaces(&self, clave: &str) -> Resultado<Vec<EnlaceRemoto>> {
        let clave = validar_clave(clave)?;
        let v: Value = self
            .json(self.http.get(self.url(&format!("rest/api/3/issue/{clave}/remotelink"))?))
            .await?;
        Ok(v.as_array()
            .map(|a| {
                a.iter()
                    .map(|e| EnlaceRemoto {
                        titulo: e.pointer("/object/title").and_then(Value::as_str).unwrap_or("").to_string(),
                        url: e.pointer("/object/url").and_then(Value::as_str).unwrap_or("").to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default())
    }
}

/// Convierte códigos HTTP en errores de la aplicación.
async fn revisar(resp: Response) -> Resultado<Response> {
    let estado = resp.status();
    if estado.is_success() {
        return Ok(resp);
    }
    Err(match estado {
        StatusCode::UNAUTHORIZED => ErrorApp::NoAutorizado,
        StatusCode::FORBIDDEN => ErrorApp::SinPermiso,
        StatusCode::TOO_MANY_REQUESTS => ErrorApp::Limite,
        _ => {
            let cuerpo = resp.text().await.unwrap_or_default();
            ErrorApp::Jira { estado: estado.as_u16(), detalle: resumir_error(&cuerpo) }
        }
    })
}

/// Jira devuelve `{"errorMessages": [...], "errors": {...}}`; si no, se recorta el texto.
fn resumir_error(cuerpo: &str) -> String {
    if let Ok(v) = serde_json::from_str::<Value>(cuerpo) {
        let mut partes: Vec<String> = v
            .get("errorMessages")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|m| m.as_str().map(String::from)).collect())
            .unwrap_or_default();
        if let Some(obj) = v.get("errors").and_then(Value::as_object) {
            partes.extend(obj.iter().map(|(k, m)| format!("{k}: {}", m.as_str().unwrap_or(""))));
        }
        if !partes.is_empty() {
            return partes.join(" · ");
        }
    }
    cuerpo.chars().take(300).collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::auth::Sitio;
    use secrecy::SecretString;
    use wiremock::matchers::{body_partial_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn cliente(servidor: &MockServer) -> JiraHttp {
        let cred = Credenciales::token_clasico(
            Sitio::parse("prueba.atlassian.net").unwrap(),
            "a@b.com",
            SecretString::from("tok"),
        )
        .unwrap();
        JiraHttp::con_base(cred, Url::parse(&format!("{}/", servidor.uri())).unwrap())
    }

    #[tokio::test]
    async fn yo_envia_basic_auth_y_lee_usuario() {
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
        let u = cliente(&s).await.yo().await.unwrap();
        assert_eq!(u.account_id, "abc");
        assert_eq!(u.nombre, "Persona de Prueba");
    }

    #[tokio::test]
    async fn codigo_401_es_no_autorizado_y_403_sin_permiso() {
        let s = MockServer::start().await;
        Mock::given(path("/rest/api/3/myself")).respond_with(ResponseTemplate::new(401)).mount(&s).await;
        Mock::given(path("/rest/api/3/issue/APP-1/remotelink")).respond_with(ResponseTemplate::new(403)).mount(&s).await;
        let c = cliente(&s).await;
        assert!(matches!(c.yo().await, Err(ErrorApp::NoAutorizado)));
        assert!(matches!(c.enlaces("APP-1").await, Err(ErrorApp::SinPermiso)));
    }

    #[tokio::test]
    async fn error_400_muestra_mensaje_de_jira() {
        let s = MockServer::start().await;
        Mock::given(path("/rest/api/3/search/jql"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({"errorMessages": ["JQL inválido"], "errors": {}})))
            .mount(&s)
            .await;
        match cliente(&s).await.buscar("x", &["summary"], None, 100).await {
            Err(ErrorApp::Jira { estado, detalle }) => {
                assert_eq!(estado, 400);
                assert_eq!(detalle, "JQL inválido");
            }
            otro => panic!("esperaba error de Jira, llegó {otro:?}"),
        }
    }

    #[tokio::test]
    async fn busqueda_pagina_con_next_page_token() {
        let s = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/rest/api/3/search/jql"))
            .and(body_partial_json(json!({"jql": "q", "maxResults": 50})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "issues": [{"key": "APP-1", "fields": {}}], "nextPageToken": "T2", "isLast": false
            })))
            .mount(&s)
            .await;
        let p = cliente(&s).await.buscar("q", &["summary"], None, 50).await.unwrap();
        assert_eq!(p.issues.len(), 1);
        assert_eq!(p.siguiente.as_deref(), Some("T2"));
    }

    #[tokio::test]
    async fn worklogs_recorre_paginas() {
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
        let todos = cliente(&s).await.worklogs("APP-1", 0).await.unwrap();
        assert_eq!(todos.len(), 3);
    }

    #[tokio::test]
    async fn confluence_arma_url_absoluta_y_usa_titulo_limpio() {
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
        let p = cliente(&s).await.confluence("type = page").await.unwrap();
        assert_eq!(p[0].titulo, "Documento de Despliegue - APP-1 - Ajuste");
        assert_eq!(p[0].url, "https://prueba.atlassian.net/wiki/spaces/DEV/pages/1");
    }

    #[tokio::test]
    async fn clave_invalida_no_llega_a_la_red() {
        let s = MockServer::start().await;
        let c = cliente(&s).await;
        assert!(matches!(c.worklogs("../../myself", 0).await, Err(ErrorApp::ClaveInvalida(_))));
        assert!(s.received_requests().await.unwrap().is_empty());
    }
}
