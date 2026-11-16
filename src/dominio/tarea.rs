//! Campos derivados de cada tarea (`prep()` de `ui/app.js`).
//!
//! La entrada puede traer campos que Rust no conoce (`hasPR`, `docChecked`…): se conservan tal
//! cual y se devuelven junto a los derivados. Los campos se leen del JSON con la misma tolerancia
//! que el JS: lo ausente o de otro tipo cuenta como «sin valor».

use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;
use serde::Serialize;
use serde_json::{Map, Value};

use super::calendario::{date_only, instante_ms, local_date, monday_of, redondeo_js};
use super::tablero::{resolver_tablero, DatosTablero, SprintDato};
use crate::error::{ErrorApp, Resultado};

const DIA_MS: f64 = 864e5;

/// `NO_DOC_RE`: tareas que no llevan documento de despliegue.
fn re_sin_documento() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)reuni[oó]n|capacitaci[oó]n|revisi[oó]n de tarea|soporte|an[aá]lisis").unwrap())
}

/// Registro de horas con sus campos derivados.
#[derive(Debug, Clone)]
pub struct RegistroCalc {
    pub s: i64,
    pub day: String,
    pub weekend: bool,
    pub wk: Option<String>,
}

/// Lo que los cálculos de semana, horas y tablero necesitan de una tarea ya derivada.
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
    pub worklogs: Vec<RegistroCalc>,
}

/// Una tarea de la entrada más los derivados de `prep()`.
#[derive(Debug, Clone, Serialize)]
pub struct TareaDerivada {
    /// Todos los campos recibidos (también los desconocidos) más los derivados.
    #[serde(flatten)]
    pub campos: Map<String, Value>,
    #[serde(skip)]
    pub c: Calc,
}

fn texto<'a>(m: &'a Map<String, Value>, k: &str) -> Option<&'a str> {
    m.get(k).and_then(Value::as_str)
}

/// Texto no vacío (en JS, la cadena vacía es «falsa»).
fn texto_no_vacio<'a>(m: &'a Map<String, Value>, k: &str) -> Option<&'a str> {
    texto(m, k).filter(|s| !s.is_empty())
}

fn entero(m: &Map<String, Value>, k: &str) -> i64 {
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

fn datos_tablero(m: &Map<String, Value>) -> DatosTablero {
    let sprints = m
        .get("sprints")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_object)
                .map(|s| SprintDato {
                    id: entero(s, "id"),
                    end: texto(s, "end").unwrap_or("").to_string(),
                    name: texto(s, "name").map(String::from),
                    nombre_visible: visible(s.get("name")),
                    board_id: s.get("boardId").and_then(Value::as_i64),
                    board_id_visible: visible(s.get("boardId")),
                })
                .collect()
        })
        .unwrap_or_default();
    DatosTablero {
        project: texto(m, "project").map(String::from),
        tipo: texto(m, "type").map(String::from),
        comp: texto(m, "comp").map(String::from),
        subtask: m.get("subtask").and_then(Value::as_bool).unwrap_or(false),
        parent: texto_no_vacio(m, "parent").map(String::from),
        sprints,
    }
}

/// `Math.round((new Date(res) - new Date(due)) / DAY)`; `None` si alguna fecha no se entiende (NaN).
fn dias_de_atraso(res: &str, due: &str) -> Option<i64> {
    let (a, b) = (instante_ms(res)?, instante_ms(due)?);
    Some(redondeo_js((a - b) as f64 / DIA_MS) as i64)
}

fn derivar_registro(w: &Value) -> Resultado<(Value, RegistroCalc)> {
    let mut mapa = w.as_object().cloned().ok_or_else(|| ErrorApp::Formato("Un registro de horas no es un objeto.".into()))?;
    let d = texto(&mapa, "d").ok_or_else(|| ErrorApp::Formato("Un registro de horas no trae la fecha `d`.".into()))?;
    let day = local_date(d)?;
    let weekend = super::calendario::dia_semana(&day)? >= 5;
    let monday = monday_of(&day)?;
    let wk = if weekend { None } else { Some(monday.clone()) };
    mapa.insert("day".into(), day.clone().into());
    mapa.insert("weekend".into(), weekend.into());
    mapa.insert("monday".into(), monday.into());
    mapa.insert("wk".into(), wk.clone().map_or(Value::Null, Value::from));
    let s = entero(&mapa, "s");
    Ok((Value::Object(mapa), RegistroCalc { s, day, weekend, wk }))
}

/// Calcula los derivados de todas las tareas, en el mismo orden de la entrada.
pub fn derivar(tareas: &[Map<String, Value>]) -> Resultado<Vec<TareaDerivada>> {
    let datos: Vec<DatosTablero> = tareas.iter().map(datos_tablero).collect();
    // `Object.fromEntries`: con claves repetidas gana la última.
    let mut por_clave: HashMap<&str, &DatosTablero> = HashMap::new();
    for (t, d) in tareas.iter().zip(&datos) {
        if let Some(k) = texto(t, "key") {
            por_clave.insert(k, d);
        }
    }

    let mut salida = Vec::with_capacity(tareas.len());
    for (t, d) in tareas.iter().zip(&datos) {
        let mut campos = t.clone();
        let r = resolver_tablero(d, &por_clave, 0);

        let mut wls = Vec::new();
        let mut wls_json = Vec::new();
        for w in t.get("worklogs").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default() {
            let (json, calc) = derivar_registro(w)?;
            wls_json.push(json);
            wls.push(calc);
        }
        if t.contains_key("worklogs") {
            campos.insert("worklogs".into(), Value::Array(wls_json));
        }

        let cat = texto(t, "cat").map(String::from);
        let es_done = cat.as_deref() == Some("done");
        let fin_day = date_only(texto(t, "finDev"))?;
        let res_day = match &fin_day {
            Some(f) => Some(f.clone()),
            None if es_done => date_only(texto_no_vacio(t, "resolved").or(texto(t, "scd")))?,
            None => None,
        };
        let no_fin = fin_day.is_none() && es_done;
        let due = texto_no_vacio(t, "due").map(String::from);
        let late = match (&res_day, &due) {
            (Some(r), Some(d)) => dias_de_atraso(r, d),
            _ => None,
        };
        let has_wl = !wls.is_empty();
        let need_doc = match t.get("hasPR") {
            Some(v) => v.clone(),
            None => Value::Bool(
                !re_sin_documento().is_match(texto(t, "summary").unwrap_or("")) && texto(t, "type") != Some("Subtarea"),
            ),
        };
        let cre_day = date_only(texto(t, "created"))?;

        let nulo = |o: &Option<String>| o.clone().map_or(Value::Null, Value::from);
        campos.insert("board".into(), r.board.clone().into());
        campos.insert("boardHow".into(), r.how.clone().into());
        campos.insert("finDay".into(), nulo(&fin_day));
        campos.insert("resDay".into(), nulo(&res_day));
        campos.insert("noFin".into(), no_fin.into());
        campos.insert("late".into(), late.map_or(Value::Null, Value::from));
        campos.insert("hasWl".into(), has_wl.into());
        campos.insert("needDoc".into(), need_doc.clone());
        campos.insert("creDay".into(), nulo(&cre_day));

        let c = Calc {
            key: texto(t, "key").unwrap_or_default().to_string(),
            summary: texto(t, "summary").unwrap_or_default().to_string(),
            status: texto(t, "status").map(String::from),
            cat,
            est: entero(t, "est"),
            spent: entero(t, "spent"),
            due,
            updated: texto(t, "updated").map(String::from),
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
        salida.push(TareaDerivada { campos, c });
    }
    Ok(salida)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use serde_json::json;

    fn tarea(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    fn una(v: Value) -> TareaDerivada {
        derivar(&[tarea(v)]).unwrap().remove(0)
    }

    #[test]
    fn tarea_sin_fechas_ni_horas() {
        let t = una(json!({ "key": "A-1", "summary": "Hacer algo", "type": "Tarea", "cat": "new", "worklogs": [], "est": 0, "spent": 0 }));
        assert_eq!(t.campos["finDay"], Value::Null);
        assert_eq!(t.campos["resDay"], Value::Null);
        assert_eq!(t.campos["creDay"], Value::Null);
        assert_eq!(t.campos["late"], Value::Null);
        assert_eq!(t.campos["noFin"], json!(false));
        assert_eq!(t.campos["hasWl"], json!(false));
        assert_eq!(t.campos["needDoc"], json!(true));
        assert_eq!(t.campos["board"], "Sin tablero");
    }

    #[test]
    fn resolucion_y_atraso() {
        let t = una(json!({ "key": "A-1", "summary": "x", "cat": "done", "finDev": "2026-11-12T10:00:00.000-05:00",
            "due": "2026-11-10", "created": "2026-11-01T02:00:00.000Z", "worklogs": [] }));
        assert_eq!(t.campos["finDay"], "2026-11-12");
        assert_eq!(t.campos["resDay"], "2026-11-12");
        assert_eq!(t.campos["late"], json!(2));
        assert_eq!(t.campos["noFin"], json!(false));
        assert_eq!(t.campos["creDay"], "2026-10-31");

        // Sin Fin desarrollo y concluida: usa resolved, o scd si no hay.
        let t = una(json!({ "key": "A-2", "cat": "done", "resolved": "2026-11-09T12:00:00.000-05:00", "scd": "2026-11-11", "due": "2026-11-12", "worklogs": [] }));
        assert_eq!(t.campos["resDay"], "2026-11-09");
        assert_eq!(t.campos["noFin"], json!(true));
        assert_eq!(t.campos["late"], json!(-3));
        let t = una(json!({ "key": "A-3", "cat": "done", "resolved": null, "scd": "2026-11-11", "worklogs": [] }));
        assert_eq!(t.campos["resDay"], "2026-11-11");
        // Abierta sin Fin desarrollo: sin resDay aunque tenga resolved.
        let t = una(json!({ "key": "A-4", "cat": "indeterminate", "resolved": "2026-11-11", "worklogs": [] }));
        assert_eq!(t.campos["resDay"], Value::Null);
    }

    #[test]
    fn registros_de_fin_de_semana_no_tienen_semana() {
        let t = una(json!({ "key": "A-1", "worklogs": [
            { "d": "2026-11-13T09:00:00.000-05:00", "s": 3600, "a": "x" },
            { "d": "2026-11-14T09:00:00.000-05:00", "s": 1800, "a": null },
            { "d": "2026-11-15T03:00:00.000Z", "s": 60 }
        ]}));
        let w = t.campos["worklogs"].as_array().unwrap();
        assert_eq!(w[0]["day"], "2026-11-13");
        assert_eq!(w[0]["wk"], "2026-11-09");
        assert_eq!(w[0]["monday"], "2026-11-09");
        assert_eq!(w[0]["weekend"], json!(false));
        assert_eq!(w[0]["a"], "x");
        assert_eq!(w[1]["weekend"], json!(true));
        assert_eq!(w[1]["wk"], Value::Null);
        assert_eq!(w[1]["monday"], "2026-11-09");
        // 03:00 UTC del domingo es sábado a la noche en Ecuador.
        assert_eq!(w[2]["day"], "2026-11-14");
        assert!(t.c.has_wl);
        assert!(t.c.worklogs[1].wk.is_none());
    }

    #[test]
    fn need_doc_por_texto_tipo_y_has_pr() {
        let n = |v: Value| una(v).campos["needDoc"].clone();
        assert_eq!(n(json!({ "key": "A", "summary": "Reunión semanal", "worklogs": [] })), json!(false));
        assert_eq!(n(json!({ "key": "A", "summary": "REVISIÓN DE TAREA 5", "worklogs": [] })), json!(false));
        assert_eq!(n(json!({ "key": "A", "summary": "Análisis de datos", "worklogs": [] })), json!(false));
        assert_eq!(n(json!({ "key": "A", "summary": "Soporte a cliente", "worklogs": [] })), json!(false));
        assert_eq!(n(json!({ "key": "A", "summary": "Pantalla", "type": "Subtarea", "worklogs": [] })), json!(false));
        assert_eq!(n(json!({ "key": "A", "summary": "Pantalla", "type": "Tarea", "worklogs": [] })), json!(true));
        // hasPR manda cuando está definido, incluso sobre el texto y el tipo.
        assert_eq!(n(json!({ "key": "A", "summary": "Reunión", "hasPR": true, "worklogs": [] })), json!(true));
        assert_eq!(n(json!({ "key": "A", "summary": "Pantalla", "hasPR": false, "worklogs": [] })), json!(false));
    }

    #[test]
    fn conserva_campos_desconocidos() {
        let t = una(json!({ "key": "A", "docChecked": true, "extra": { "x": [1, 2] }, "worklogs": [{ "d": "2026-11-10", "s": 1, "otro": 5 }] }));
        assert_eq!(t.campos["docChecked"], json!(true));
        assert_eq!(t.campos["extra"], json!({ "x": [1, 2] }));
        assert_eq!(t.campos["worklogs"][0]["otro"], json!(5));
        // Y los campos que no estaban siguen ausentes.
        assert!(!t.campos.contains_key("hasPR"));
        assert!(!t.campos.contains_key("sp"));
    }

    #[test]
    fn subtarea_toma_el_tablero_del_padre() {
        let r = derivar(&[
            tarea(json!({ "key": "P-1", "project": "ADN", "worklogs": [] })),
            tarea(json!({ "key": "P-2", "subtask": true, "parent": "P-1", "worklogs": [] })),
        ])
        .unwrap();
        assert_eq!(r[1].campos["board"], "Tablero ADN");
        assert_eq!(r[1].campos["boardHow"], "tarea padre P-1");
    }

    #[test]
    fn registro_sin_fecha_es_error_de_formato() {
        let e = derivar(&[tarea(json!({ "key": "A", "worklogs": [{ "s": 5 }] }))]).unwrap_err();
        assert_eq!(e.codigo(), "formato");
    }
}
