//! Acceso a Jira y Confluence: fuente real (HTTP) o simulada, y operaciones de alto nivel.
//!
//! Los comandos de Tauri trabajan siempre con `Fuente`; la elección entre la API real y los
//! datos ficticios se hace una sola vez al iniciar sesión.

pub mod http;
pub mod modelo;
pub mod simulado;

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use futures::stream::{self, StreamExt};
use regex::Regex;
use serde::Serialize;
use serde_json::Value;

use crate::error::{ErrorApp, Resultado};
use modelo::{Registro, Tarea, Usuario};

/// Una página de resultados de `POST /rest/api/3/search/jql`.
#[derive(Debug)]
pub struct PaginaBusqueda {
    pub issues: Vec<Value>,
    pub siguiente: Option<String>,
}

/// Una página de Confluence encontrada por CQL (`url` absoluta).
pub struct PaginaWiki {
    pub titulo: String,
    pub url: String,
}

/// Un enlace remoto de una tarea de Jira.
pub struct EnlaceRemoto {
    pub titulo: String,
    pub url: String,
}

pub const JQL_TAREAS: &str = "assignee = currentUser() OR worklogAuthor = currentUser() ORDER BY updated DESC";
pub const JQL_PR: &str =
    "(assignee = currentUser() OR worklogAuthor = currentUser()) AND development[pullrequests].all > 0";
pub const CAMPOS: &[&str] = &[
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

/// Una clave de Jira válida: `APP-123`. Evita que texto arbitrario llegue a una URL o a un JQL.
pub fn validar_clave(clave: &str) -> Resultado<&str> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^[A-Z][A-Z0-9_]{0,19}-[0-9]{1,9}$").expect("regex válida"));
    if re.is_match(clave) {
        Ok(clave)
    } else {
        Err(ErrorApp::ClaveInvalida(clave.chars().take(40).collect()))
    }
}

/// Despacho por enum (sin async-trait): cada método hace `match` y delega.
pub enum Fuente {
    Http(http::JiraHttp),
    Simulado(simulado::JiraSimulado),
}

impl Fuente {
    pub async fn yo(&self) -> Resultado<Usuario> {
        match self {
            Fuente::Http(h) => h.yo().await,
            Fuente::Simulado(s) => s.yo().await,
        }
    }

    pub async fn buscar(
        &self,
        jql: &str,
        campos: &[&str],
        siguiente: Option<&str>,
        tam: u32,
    ) -> Resultado<PaginaBusqueda> {
        match self {
            Fuente::Http(h) => h.buscar(jql, campos, siguiente, tam).await,
            Fuente::Simulado(s) => s.buscar(jql, campos, siguiente, tam).await,
        }
    }

    /// Registros de horas de la tarea (JSON crudo de Jira), de cualquier autor.
    pub async fn worklogs(&self, clave: &str, desde_ms: i64) -> Resultado<Vec<Value>> {
        match self {
            Fuente::Http(h) => h.worklogs(clave, desde_ms).await,
            Fuente::Simulado(s) => s.worklogs(clave, desde_ms).await,
        }
    }

    pub async fn confluence(&self, cql: &str) -> Resultado<Vec<PaginaWiki>> {
        match self {
            Fuente::Http(h) => h.confluence(cql).await,
            Fuente::Simulado(s) => s.confluence(cql).await,
        }
    }

    pub async fn enlaces(&self, clave: &str) -> Resultado<Vec<EnlaceRemoto>> {
        match self {
            Fuente::Http(h) => h.enlaces(clave).await,
            Fuente::Simulado(s) => s.enlaces(clave).await,
        }
    }
}

/// Recorre las páginas de una búsqueda (con tope) y devuelve todos los `issues`.
async fn buscar_todo(f: &Fuente, jql: &str, campos: &[&str], tam: u32, max_paginas: usize) -> Resultado<Vec<Value>> {
    let mut todos = Vec::new();
    let mut token: Option<String> = None;
    for _ in 0..max_paginas {
        let pagina = f.buscar(jql, campos, token.as_deref(), tam).await?;
        todos.extend(pagina.issues);
        token = pagina.siguiente;
        if token.is_none() {
            break;
        }
    }
    Ok(todos)
}

fn claves_de(issues: &[Value]) -> Vec<String> {
    issues.iter().filter_map(|v| v.get("key").and_then(Value::as_str).map(String::from)).collect()
}

/// Tareas del usuario (asignadas o con horas suyas), ya convertidas al modelo de la interfaz.
pub async fn cargar_tareas(f: &Fuente, yo: Option<&str>) -> Resultado<Vec<Tarea>> {
    let issues = buscar_todo(f, JQL_TAREAS, CAMPOS, 100, 20).await?;
    Ok(issues.iter().filter_map(|v| Tarea::desde_jira(v, yo)).collect())
}

/// Claves de las tareas donde el usuario registró horas desde `desde` (`AAAA-MM-DD`).
pub async fn claves_con_horas(f: &Fuente, desde: &str) -> Resultado<Vec<String>> {
    let fecha = chrono::NaiveDate::parse_from_str(desde, "%Y-%m-%d")
        .ok()
        .filter(|_| desde.len() == 10)
        .ok_or_else(|| ErrorApp::Formato(format!("fecha no válida (se esperaba AAAA-MM-DD): {}", desde.chars().take(20).collect::<String>())))?;
    let jql = format!("worklogAuthor = currentUser() AND worklogDate >= \"{}\"", fecha.format("%Y-%m-%d"));
    Ok(claves_de(&buscar_todo(f, &jql, &["summary"], 100, 10).await?))
}

/// Claves de las tareas del usuario que tienen pull requests en el panel de desarrollo.
pub async fn claves_con_pr(f: &Fuente) -> Resultado<Vec<String>> {
    Ok(claves_de(&buscar_todo(f, JQL_PR, &["summary"], 100, 20).await?))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultadoHoras {
    pub por_clave: HashMap<String, Vec<Registro>>,
    pub fallidas: Vec<String>,
    pub error: Option<String>,
}

/// Lee los registros de horas propios de cada tarea (4 consultas a la vez).
///
/// Una tarea que falla se informa en `fallidas`; si falla por credenciales, o fallan todas, es un error.
pub async fn leer_horas(f: &Fuente, claves: &[String], desde_ms: i64, yo: Option<&str>) -> Resultado<ResultadoHoras> {
    for c in claves {
        validar_clave(c)?;
    }
    let mut unicas: Vec<String> = Vec::new();
    for c in claves {
        if !unicas.contains(c) {
            unicas.push(c.clone());
        }
    }
    let resultados: Vec<(String, Resultado<Vec<Value>>)> = stream::iter(unicas.iter().cloned())
        .map(|c| async move {
            let r = f.worklogs(&c, desde_ms).await;
            (c, r)
        })
        .buffer_unordered(4)
        .collect()
        .await;

    let mut por_clave = HashMap::new();
    let mut fallidas = Vec::new();
    let mut ultimo: Option<ErrorApp> = None;
    for (clave, r) in resultados {
        match r {
            Ok(valores) => {
                let registros: Vec<Registro> =
                    valores.iter().filter_map(Registro::desde_jira).filter(|r| r.es_de(yo)).collect();
                por_clave.insert(clave, registros);
            }
            Err(ErrorApp::NoAutorizado) => return Err(ErrorApp::NoAutorizado),
            Err(e) => {
                fallidas.push(clave);
                ultimo = Some(e);
            }
        }
    }
    fallidas.sort();
    if !unicas.is_empty() && fallidas.len() == unicas.len() {
        if let Some(e) = ultimo {
            return Err(e);
        }
    }
    Ok(ResultadoHoras { por_clave, fallidas, error: ultimo.map(|e| e.to_string()) })
}

/// Lo que hace la interfaz al cargar (sin documentos ni PR): tareas + horas propias desde el lunes de
/// `hoy - 98 días` (instante 00:00 de ese lunes en UTC-5). Cada tarea queda con `worklogs`, `wlTotal` y,
/// si `spent` era 0, `spent` = suma de sus registros. Devuelve el formato de entrada de `calcular_vista`.
pub async fn cargar_para_resumen(
    f: &Fuente,
    yo: Option<&str>,
    hoy: chrono::NaiveDate,
) -> Resultado<Vec<serde_json::Map<String, Value>>> {
    use chrono::{Datelike, Duration};

    let tareas = cargar_tareas(f, yo).await?;
    let mut tareas: Vec<serde_json::Map<String, Value>> = tareas
        .iter()
        .map(|t| match serde_json::to_value(t) {
            Ok(Value::Object(m)) => Ok(m),
            _ => Err(ErrorApp::Formato("tarea no serializable".into())),
        })
        .collect::<Resultado<_>>()?;

    let atras = hoy - Duration::days(98);
    let desde_lunes = atras - Duration::days(i64::from(atras.weekday().num_days_from_monday()));
    let desde = desde_lunes.format("%Y-%m-%d").to_string();
    let despues_ms = desde_lunes
        .and_hms_opt(5, 0, 0)
        .ok_or_else(|| ErrorApp::Formato("hora no válida".into()))?
        .and_utc()
        .timestamp_millis();

    let claves = claves_con_horas(f, &desde).await?;
    let horas = if claves.is_empty() { None } else { Some(leer_horas(f, &claves, despues_ms, yo).await?) };

    for t in &mut tareas {
        let clave = t.get("key").and_then(Value::as_str).unwrap_or_default().to_string();
        let wl = match horas.as_ref().and_then(|h| h.por_clave.get(&clave)) {
            Some(r) => serde_json::to_value(r).map_err(|e| ErrorApp::Formato(e.to_string()))?,
            None => Value::Array(Vec::new()),
        };
        let suma: i64 = wl.as_array().map(|a| a.iter().filter_map(|w| w["s"].as_i64()).sum()).unwrap_or(0);
        t.insert("wlTotal".into(), Value::from(wl.as_array().map_or(0, Vec::len)));
        if t.get("spent").and_then(Value::as_i64).unwrap_or(0) == 0 {
            t.insert("spent".into(), Value::from(suma));
        }
        t.insert("worklogs".into(), wl);
    }
    Ok(tareas)
}

/// Busca el «Documento de Despliegue» de cada tarea: primero en Confluence (por título) y, para las
/// que no aparecen, en los enlaces remotos de la tarea. Devuelve `clave → url`.
pub async fn buscar_documentos(f: &Fuente, claves: &[String]) -> Resultado<HashMap<String, String>> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(?i)Despliegue\s*-?\s*([A-Z][A-Z0-9]+-\d+)").expect("regex válida"));

    for c in claves {
        validar_clave(c)?;
    }
    let mut unicas: Vec<&str> = Vec::new();
    for c in claves {
        if !unicas.contains(&c.as_str()) {
            unicas.push(c);
        }
    }

    let mut docs: HashMap<String, String> = HashMap::new();
    for grupo in unicas.chunks(25) {
        let filtros: Vec<String> = grupo.iter().map(|c| format!("title ~ \"\\\"{c}\\\"\"")).collect();
        let cql = format!("type = page AND title ~ \"Despliegue\" AND ({})", filtros.join(" OR "));
        for pagina in f.confluence(&cql).await? {
            if let Some(m) = re.captures(&pagina.titulo) {
                let clave = m[1].to_uppercase();
                if unicas.contains(&clave.as_str()) {
                    docs.entry(clave).or_insert(pagina.url);
                }
            }
        }
    }

    // Respaldo: el buscador de Confluence tarda en indexar páginas nuevas, pero el enlace remoto
    // de la tarea ya existe. Los errores de este paso se ignoran (queda como «sin documento»).
    let faltan: Vec<String> =
        unicas.iter().copied().filter(|c| !docs.contains_key(*c)).take(60).map(String::from).collect();
    let hallados: Vec<Option<(String, String)>> = stream::iter(faltan)
        .map(|c| async move {
            let enlaces = f.enlaces(&c).await.ok()?;
            let mejor = enlaces
                .into_iter()
                .filter(|e| e.titulo.to_lowercase().contains("despliegue") || e.url.contains("Documento+de+Despliegue"))
                .min_by_key(|e| !e.url.contains("/spaces/"))?;
            Some((c, mejor.url))
        })
        .buffer_unordered(5)
        .collect()
        .await;
    docs.extend(hallados.into_iter().flatten());
    // Solo claves pedidas (por construcción), sin duplicados.
    let pedidas: HashSet<&str> = unicas.iter().copied().collect();
    docs.retain(|k, _| pedidas.contains(k.as_str()));
    Ok(docs)
}

#[cfg(test)]
mod pruebas {
    use super::simulado::JiraSimulado;
    use super::*;
    use chrono::NaiveDate;

    fn hoy() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
    }

    fn sim(valido: bool) -> Fuente {
        Fuente::Simulado(JiraSimulado::nuevo("prueba@ejemplo.com", valido, hoy()))
    }

    #[test]
    fn valida_claves() {
        assert_eq!(validar_clave("APP-123").unwrap(), "APP-123");
        assert!(validar_clave("A1_B-9").is_ok());
        for mala in ["app-1", "APP-", "-1", "APP-1x", "../../myself", "APP-1\n", "", "1APP-2", "APP-1234567890"] {
            assert!(matches!(validar_clave(mala), Err(ErrorApp::ClaveInvalida(_))), "{mala:?}");
        }
    }

    #[tokio::test]
    async fn cargar_tareas_pagina_sin_duplicados() {
        let f = sim(true);
        let tareas = cargar_tareas(&f, Some("sim-0001")).await.unwrap();
        let claves: HashSet<&str> = tareas.iter().map(|t| t.key.as_str()).collect();
        assert_eq!(claves.len(), tareas.len(), "hay claves repetidas");
        assert!(tareas.len() > 10, "debe ocupar más de una página de 10");
        // El simulado entrega las páginas de 10 en 10: confirma que de verdad hubo paginación.
        let primera = f.buscar(JQL_TAREAS, CAMPOS, None, 100).await.unwrap();
        assert_eq!(primera.issues.len(), 10);
        assert!(primera.siguiente.is_some());
        // Ningún registro ajeno se cuela en las tareas.
        assert!(tareas.iter().flat_map(|t| &t.worklogs).all(|w| w.a.as_deref() == Some("sim-0001")));
    }

    #[tokio::test]
    async fn leer_horas_filtra_otro_autor() {
        let f = sim(true);
        let claves = vec!["APP-401".to_string(), "AFO-101".to_string()];
        let crudo = f.worklogs("APP-401", 0).await.unwrap();
        assert!(crudo.iter().any(|w| w["author"]["accountId"] == "sim-0002"), "el simulado debe traer horas ajenas");
        let r = leer_horas(&f, &claves, 0, Some("sim-0001")).await.unwrap();
        assert!(r.fallidas.is_empty() && r.error.is_none());
        let app = &r.por_clave["APP-401"];
        assert!(!app.is_empty());
        assert!(app.len() < crudo.len());
        assert!(app.iter().all(|x| x.a.as_deref() == Some("sim-0001")));
        let js = serde_json::to_value(&r).unwrap();
        assert!(js.get("porClave").is_some());
    }

    #[tokio::test]
    async fn leer_horas_con_token_invalido_es_error() {
        let f = sim(false);
        let r = leer_horas(&f, &["APP-401".to_string()], 0, None).await;
        assert!(matches!(r, Err(ErrorApp::NoAutorizado)));
        assert!(matches!(f.yo().await, Err(ErrorApp::NoAutorizado)));
    }

    #[tokio::test]
    async fn leer_horas_rechaza_claves_invalidas() {
        let f = sim(true);
        let r = leer_horas(&f, &["APP-401".to_string(), "x y".to_string()], 0, None).await;
        assert!(matches!(r, Err(ErrorApp::ClaveInvalida(_))));
    }

    #[tokio::test]
    async fn buscar_documentos_por_confluence_y_por_enlace() {
        let f = sim(true);
        let con_pr = claves_con_pr(&f).await.unwrap();
        let docs = buscar_documentos(&f, &con_pr).await.unwrap();
        // Por Confluence.
        assert!(docs["AFO-102"].contains("/wiki/spaces/"));
        // Solo como enlace remoto (el respaldo).
        assert!(docs["APP-402"].contains("Documento+de+Despliegue"));
        // Sin documento.
        assert!(!docs.contains_key("APP-403"));
        // Solo claves pedidas.
        assert!(docs.keys().all(|k| con_pr.contains(k)));
        let parcial = buscar_documentos(&f, &["AFO-102".to_string()]).await.unwrap();
        assert_eq!(parcial.len(), 1);
    }

    #[tokio::test]
    async fn claves_con_pr_solo_trae_tareas_con_pr() {
        let f = sim(true);
        let con_pr = claves_con_pr(&f).await.unwrap();
        assert!(con_pr.contains(&"APP-403".to_string()));
        assert!(!con_pr.contains(&"OPE-701".to_string()), "la reunión no tiene PR");
    }

    #[tokio::test]
    async fn claves_con_horas_valida_la_fecha() {
        let f = sim(true);
        for mala in ["2026-13-01", "ayer", "2026-1-5", "", "2026-11-16'; DROP", "16/11/2026"] {
            assert!(matches!(claves_con_horas(&f, mala).await, Err(ErrorApp::Formato(_))), "{mala:?}");
        }
        let todas = claves_con_horas(&f, "2020-01-01").await.unwrap();
        let recientes = claves_con_horas(&f, "2026-11-10").await.unwrap();
        assert!(todas.contains(&"OPE-701".to_string()));
        assert!(!recientes.is_empty() && recientes.len() < todas.len());
        assert!(!todas.contains(&"ADN-301".to_string()), "ADN-301 no tiene horas");
    }

    fn fecha_pruebas() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 13).unwrap()
    }

    fn sim_viernes(valido: bool) -> Fuente {
        Fuente::Simulado(JiraSimulado::nuevo("a@b.com", valido, fecha_pruebas()))
    }

    #[tokio::test]
    async fn cargar_para_resumen_devuelve_tareas_con_horas_propias() {
        use chrono::{Datelike, Duration};
        let f = sim_viernes(true);
        let tareas = cargar_para_resumen(&f, Some("sim-0001"), fecha_pruebas()).await.unwrap();
        assert!(!tareas.is_empty());
        let atras = fecha_pruebas() - Duration::days(98);
        let desde = (atras - Duration::days(i64::from(atras.weekday().num_days_from_monday()))).format("%Y-%m-%d").to_string();
        let mut hubo_registros = false;
        for t in &tareas {
            assert!(t.get("key").and_then(Value::as_str).is_some());
            assert!(!t.contains_key("hasPR"));
            let wl = t.get("worklogs").and_then(Value::as_array).expect("worklogs es un arreglo");
            for w in wl {
                hubo_registros = true;
                assert_ne!(w["a"].as_str(), Some("sim-0002"), "registro ajeno");
                assert!(w["d"].as_str().unwrap() >= desde.as_str(), "registro anterior al lunes de hoy - 98 días");
            }
        }
        assert!(hubo_registros);
    }

    #[tokio::test]
    async fn cargar_para_resumen_con_token_invalido_es_no_autorizado() {
        let f = sim_viernes(false);
        let r = cargar_para_resumen(&f, None, fecha_pruebas()).await;
        assert!(matches!(r, Err(ErrorApp::NoAutorizado)));
    }

    #[tokio::test]
    async fn cargar_para_resumen_completa_spent_con_la_suma_de_registros() {
        let f = sim_viernes(true);
        let originales = cargar_tareas(&f, Some("sim-0001")).await.unwrap();
        let sin_gastado: Vec<&str> = originales.iter().filter(|t| t.spent == 0).map(|t| t.key.as_str()).collect();
        assert!(!sin_gastado.is_empty(), "el simulado debe traer alguna tarea sin tiempo gastado");
        let tareas = cargar_para_resumen(&f, Some("sim-0001"), fecha_pruebas()).await.unwrap();
        for t in tareas.iter().filter(|t| sin_gastado.contains(&t["key"].as_str().unwrap())) {
            let suma: i64 = t["worklogs"].as_array().unwrap().iter().filter_map(|w| w["s"].as_i64()).sum();
            assert_eq!(t["spent"].as_i64(), Some(suma), "{}", t["key"]);
        }
    }
}
