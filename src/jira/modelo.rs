//! Modelo de datos que recibe la interfaz.
//!
//! Convierte el JSON de la API REST de Jira al formato que ya usaba el artifact
//! (los mismos nombres de campo: `key`, `summary`, `cat`, `finDev`, `worklogs`…),
//! para que el código de la interfaz casi no cambie. Las fechas de Jira
//! (`2026-11-10T08:30:00.000-0500`) se pasan a RFC 3339 (`…-05:00`), que todos los
//! motores web interpretan igual.

use chrono::{DateTime, SecondsFormat};
use serde::Serialize;
use serde_json::Value;

/// Campos personalizados de Jira de Torres & Torres.
pub const CAMPO_INICIO: &str = "customfield_10015";
pub const CAMPO_SPRINTS: &str = "customfield_10020";
pub const CAMPO_PUNTOS: &str = "customfield_10031";
pub const CAMPO_FIN_DESARROLLO: &str = "customfield_10057";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Usuario {
    pub account_id: String,
    pub nombre: String,
    pub correo: Option<String>,
}

impl Usuario {
    /// Desde `GET /rest/api/3/myself`.
    pub fn desde_jira(v: &Value) -> Option<Usuario> {
        Some(Usuario {
            account_id: v.get("accountId")?.as_str()?.to_string(),
            nombre: texto(v, "/displayName").unwrap_or_default(),
            correo: texto(v, "/emailAddress"),
        })
    }
}

/// Un registro de horas: día de inicio (`d`), segundos (`s`) y autor (`a`).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Registro {
    pub d: String,
    pub s: i64,
    pub a: Option<String>,
}

impl Registro {
    pub fn desde_jira(v: &Value) -> Option<Registro> {
        Some(Registro {
            d: normalizar_fecha(v.get("started")?.as_str()?),
            s: v.get("timeSpentSeconds").and_then(Value::as_i64).unwrap_or(0),
            a: texto(v, "/author/accountId"),
        })
    }

    /// Solo cuentan los registros propios. Si no se conoce el usuario o el autor, se conserva.
    pub fn es_de(&self, yo: Option<&str>) -> bool {
        match (yo, self.a.as_deref()) {
            (Some(y), Some(a)) => a == y,
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
    fn desde_jira(v: &Value) -> Option<Sprint> {
        Some(Sprint {
            id: v.get("id")?.as_i64()?,
            name: texto(v, "/name").unwrap_or_default(),
            board_id: v.get("boardId").and_then(Value::as_i64),
            state: texto(v, "/state"),
            end: texto(v, "/endDate").or_else(|| texto(v, "/completeDate")).unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tarea {
    pub key: String,
    pub summary: String,
    pub project: Option<String>,
    pub project_name: Option<String>,
    pub comp: String,
    #[serde(rename = "type")]
    pub tipo: Option<String>,
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
    pub worklogs: Vec<Registro>,
}

impl Tarea {
    /// Desde un elemento de `issues` de `POST /rest/api/3/search/jql`.
    pub fn desde_jira(v: &Value, yo: Option<&str>) -> Option<Tarea> {
        let key = v.get("key")?.as_str()?.to_string();
        let vacio = Value::Null;
        let f = v.get("fields").unwrap_or(&vacio);
        let fecha = |p: &str| texto(f, p).map(|s| normalizar_fecha(&s));
        let sprints: Vec<Sprint> = f
            .get(CAMPO_SPRINTS)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Sprint::desde_jira).collect())
            .unwrap_or_default();
        let worklogs: Vec<Registro> = f
            .pointer("/worklog/worklogs")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Registro::desde_jira).filter(|r| r.es_de(yo)).collect())
            .unwrap_or_default();
        Some(Tarea {
            summary: texto(f, "/summary").unwrap_or_default(),
            project: texto(f, "/project/key"),
            project_name: texto(f, "/project/name"),
            comp: texto(f, "/components/0/name").unwrap_or_else(|| "Sin componente".into()),
            tipo: texto(f, "/issuetype/name"),
            subtask: f.pointer("/issuetype/subtask").and_then(Value::as_bool).unwrap_or(false),
            status: texto(f, "/status/name"),
            cat: texto(f, "/status/statusCategory/key"),
            priority: texto(f, "/priority/name"),
            scd: fecha("/statuscategorychangedate"),
            created: fecha("/created"),
            updated: fecha("/updated"),
            resolved: fecha("/resolutiondate"),
            due: fecha("/duedate"),
            start: fecha(&format!("/{CAMPO_INICIO}")),
            fin_dev: fecha(&format!("/{CAMPO_FIN_DESARROLLO}")),
            est: f.get("timeoriginalestimate").and_then(Value::as_i64).unwrap_or(0),
            spent: f.get("timespent").and_then(Value::as_i64).unwrap_or(0),
            sp: f.get(CAMPO_PUNTOS).and_then(Value::as_f64),
            labels: f
                .get("labels")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|l| l.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            sprint: sprints.last().map(|s| s.name.clone()),
            sprints,
            parent: texto(f, "/parent/key"),
            wl_total: f.pointer("/worklog/total").and_then(Value::as_i64).unwrap_or(0),
            worklogs,
            key,
        })
    }
}

fn texto(v: &Value, puntero: &str) -> Option<String> {
    v.pointer(puntero).and_then(Value::as_str).map(String::from)
}

/// Fecha-hora de Jira → RFC 3339. Las fechas sin hora (`2026-11-10`) quedan igual.
pub fn normalizar_fecha(s: &str) -> String {
    DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f%z")
        .or_else(|_| DateTime::parse_from_rfc3339(s))
        .map(|d| d.to_rfc3339_opts(SecondsFormat::Millis, false))
        .unwrap_or_else(|_| s.to_string())
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use serde_json::json;

    #[test]
    fn normaliza_fechas_de_jira() {
        assert_eq!(normalizar_fecha("2026-11-10T08:30:00.000-0500"), "2026-11-10T08:30:00.000-05:00");
        assert_eq!(normalizar_fecha("2026-11-10T08:30:00-05:00"), "2026-11-10T08:30:00.000-05:00");
        assert_eq!(normalizar_fecha("2026-11-10"), "2026-11-10");
    }

    #[test]
    fn convierte_tarea_y_filtra_registros_ajenos() {
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
        let t = Tarea::desde_jira(&v, Some("yo")).unwrap();
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

        // La interfaz espera los nombres del artifact.
        let js = serde_json::to_value(&t).unwrap();
        for campo in ["finDev", "projectName", "wlTotal", "type"] {
            assert!(js.get(campo).is_some(), "falta {campo}");
        }
        assert_eq!(js["sprints"][0]["boardId"], 455);
    }

    #[test]
    fn sin_key_no_hay_tarea() {
        assert!(Tarea::desde_jira(&json!({"fields": {}}), None).is_none());
    }
}
