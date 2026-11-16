//! Estadísticas por semana, orden de tableros y última actividad
//! (`weekStats`, `boards`, `lastActivity`, `daySecs` de `ui/app.js`).

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::Serialize;

use super::calendario::{add_days, date_only};
use super::tarea::TareaDerivada;
use crate::error::Resultado;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub secs: i64,
    /// Claves de tarea con horas en la semana, en el orden de `tareas`.
    pub worked: Vec<String>,
    pub per_board: BTreeMap<String, i64>,
    pub per_day: BTreeMap<String, BTreeMap<String, i64>>,
    /// Claves de tarea concluidas en la semana, en el orden de `tareas`.
    pub resolved: Vec<String>,
    pub est: i64,
    pub spent: i64,
    pub with_due: i64,
    pub on_time: i64,
}

/// `weekStats(wk, filter)`. Los registros de fin de semana (`wk` nulo) no suman.
pub fn estadisticas_semana(tareas: &[TareaDerivada], wk: &str, filtro: &dyn Fn(&TareaDerivada) -> bool) -> Resultado<Stats> {
    let fin = add_days(wk, 4)?;
    let mut st = Stats {
        secs: 0,
        worked: Vec::new(),
        per_board: BTreeMap::new(),
        per_day: BTreeMap::new(),
        resolved: Vec::new(),
        est: 0,
        spent: 0,
        with_due: 0,
        on_time: 0,
    };
    for t in tareas.iter().filter(|t| filtro(t)) {
        let mut trabajada = false;
        for w in t.c.worklogs.iter().filter(|w| w.wk.as_deref() == Some(wk)) {
            st.secs += w.s;
            trabajada = true;
            *st.per_board.entry(t.c.board.clone()).or_insert(0) += w.s;
            *st.per_day.entry(w.day.clone()).or_default().entry(t.c.board.clone()).or_insert(0) += w.s;
        }
        if trabajada {
            st.worked.push(t.c.key.clone());
        }
        if let Some(r) = t.c.res_day.as_deref() {
            if r >= wk && r <= fin.as_str() {
                st.resolved.push(t.c.key.clone());
                st.est += t.c.est;
                st.spent += t.c.spent;
                if let Some(due) = t.c.due.as_deref() {
                    st.with_due += 1;
                    if r <= due {
                        st.on_time += 1;
                    }
                }
            }
        }
    }
    Ok(st)
}

/// `daySecs(day)`: todas las horas de un día, incluidos fines de semana.
pub fn segundos_del_dia(tareas: &[TareaDerivada], day: &str) -> i64 {
    tareas.iter().flat_map(|t| &t.c.worklogs).filter(|w| w.day == day).map(|w| w.s).sum()
}

/// `lastActivity(b)`: la fecha más reciente entre horas, resolución y actualización (`""` si no hay).
pub fn ultima_actividad(tareas: &[TareaDerivada], tablero: &str) -> Resultado<String> {
    let mut ultima = String::new();
    for t in tareas.iter().filter(|t| t.c.board == tablero) {
        for w in &t.c.worklogs {
            if w.day > ultima {
                ultima = w.day.clone();
            }
        }
        if let Some(r) = &t.c.res_day {
            if *r > ultima {
                ultima = r.clone();
            }
        }
        if let Some(u) = date_only(t.c.updated.as_deref())? {
            if u > ultima {
                ultima = u;
            }
        }
    }
    Ok(ultima)
}

/// `boards()`: nombres únicos ordenados por horas de la semana, concluidas en la semana, abiertas
/// y última actividad (todos de mayor a menor) y, al final, por nombre.
pub fn tableros(tareas: &[TareaDerivada], wk: &str) -> Resultado<Vec<String>> {
    let mut nombres: Vec<String> = Vec::new();
    for t in tareas {
        if !nombres.contains(&t.c.board) {
            nombres.push(t.c.board.clone());
        }
    }
    let mut puntaje: BTreeMap<String, (i64, i64, i64, String)> = BTreeMap::new();
    for b in &nombres {
        let s = estadisticas_semana(tareas, wk, &|t| t.c.board == *b)?;
        let abiertas = tareas.iter().filter(|t| t.c.board == *b && t.c.cat.as_deref() != Some("done")).count() as i64;
        puntaje.insert(b.clone(), (s.secs, s.resolved.len() as i64, abiertas, ultima_actividad(tareas, b)?));
    }
    nombres.sort_by(|a, b| {
        let (x, y) = (&puntaje[a], &puntaje[b]);
        y.0.cmp(&x.0)
            .then(y.1.cmp(&x.1))
            .then(y.2.cmp(&x.2))
            .then_with(|| comparar_locale(&y.3, &x.3))
            .then_with(|| comparar_locale(a, b))
    });
    Ok(nombres)
}

const PUNTUACION: &str = "_-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$";

/// Clave primaria de un carácter según el orden de ICU (lo que usa `localeCompare`):
/// espacios, signos, dígitos y letras (sin distinguir mayúsculas ni acentos).
fn primaria(c: char) -> (u8, u32) {
    if c.is_whitespace() {
        return (0, 0);
    }
    if let Some(p) = PUNTUACION.find(c) {
        return (1, p as u32);
    }
    if c.is_ascii_digit() {
        return (2, c as u32);
    }
    let base = match c.to_lowercase().next().unwrap_or(c) {
        'á' | 'à' | 'ä' | 'â' | 'ã' => 'a',
        'é' | 'è' | 'ë' | 'ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 'o',
        'ú' | 'ù' | 'ü' | 'û' => 'u',
        'ñ' => 'n',
        otro => otro,
    };
    (3, base as u32)
}

/// Aproximación de `String.prototype.localeCompare`: letras sin distinguir mayúsculas/acentos
/// primero; si empatan, los acentos; y por último la minúscula antes que la mayúscula.
pub fn comparar_locale(a: &str, b: &str) -> Ordering {
    let pa: Vec<_> = a.chars().map(primaria).collect();
    let pb: Vec<_> = b.chars().map(primaria).collect();
    pa.cmp(&pb)
        .then_with(|| {
            let acento = |s: &str| s.chars().map(|c| !c.is_ascii() && c.is_alphabetic() && primaria(c).0 == 3).collect::<Vec<_>>();
            acento(a).cmp(&acento(b))
        })
        .then_with(|| {
            let mayus = |s: &str| s.chars().map(|c| c.is_uppercase()).collect::<Vec<_>>();
            mayus(a).cmp(&mayus(b))
        })
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::dominio::tarea::derivar;
    use serde_json::{json, Value};

    fn tareas(v: Value) -> Vec<TareaDerivada> {
        let m: Vec<_> = v.as_array().unwrap().iter().map(|x| x.as_object().unwrap().clone()).collect();
        derivar(&m).unwrap()
    }

    fn todas(_: &TareaDerivada) -> bool {
        true
    }

    fn datos() -> Vec<TareaDerivada> {
        tareas(json!([
            { "key": "A-1", "project": "ADN", "cat": "done", "finDev": "2026-11-11", "due": "2026-11-10", "est": 3600, "spent": 7200,
              "worklogs": [
                { "d": "2026-11-09T10:00:00.000-05:00", "s": 3600 },
                { "d": "2026-11-14T10:00:00.000-05:00", "s": 5000 },
                { "d": "2026-11-02T10:00:00.000-05:00", "s": 100 } ] },
            { "key": "A-2", "project": "ADN", "cat": "done", "finDev": "2026-11-13", "due": "2026-11-13", "est": 1800, "spent": 1800, "worklogs": [] },
            { "key": "B-1", "project": "GPS", "type": "Tarea", "cat": "indeterminate", "est": 0, "spent": 0,
              "worklogs": [ { "d": "2026-11-10T10:00:00.000-05:00", "s": 1800 } ] },
            { "key": "B-2", "project": "GPS", "type": "Tarea", "cat": "done", "resolved": "2026-11-15T12:00:00.000-05:00", "est": 0, "spent": 0, "worklogs": [] }
        ]))
    }

    #[test]
    fn estadisticas_de_la_semana() {
        let d = datos();
        let s = estadisticas_semana(&d, "2026-11-09", &todas).unwrap();
        assert_eq!(s.secs, 5400); // sin el sábado ni la semana anterior
        assert_eq!(s.worked, ["A-1", "B-1"]);
        assert_eq!(s.resolved, ["A-1", "A-2"]); // B-2 cerró en domingo: fuera
        assert_eq!((s.est, s.spent), (5400, 9000));
        assert_eq!((s.with_due, s.on_time), (2, 1));
        assert_eq!(s.per_board["Tablero ADN"], 3600);
        assert_eq!(s.per_board["Tablero Kanban"], 1800);
        assert_eq!(s.per_day["2026-11-09"]["Tablero ADN"], 3600);
        assert!(!s.per_day.contains_key("2026-11-14"));
        // Con filtro de tablero.
        let s = estadisticas_semana(&d, "2026-11-09", &|t| t.c.board == "Tablero Kanban").unwrap();
        assert_eq!((s.secs, s.worked.len(), s.resolved.len()), (1800, 1, 0));
        // Semana sin nada.
        let s = estadisticas_semana(&d, "2026-11-16", &todas).unwrap();
        assert_eq!((s.secs, s.worked.len(), s.resolved.len()), (0, 0, 0));
    }

    #[test]
    fn segundos_del_dia_incluye_fin_de_semana() {
        let d = datos();
        assert_eq!(segundos_del_dia(&d, "2026-11-14"), 5000);
        assert_eq!(segundos_del_dia(&d, "2026-11-12"), 0);
    }

    #[test]
    fn ultima_actividad_toma_el_maximo() {
        let d = datos();
        assert_eq!(ultima_actividad(&d, "Tablero ADN").unwrap(), "2026-11-14");
        assert_eq!(ultima_actividad(&d, "Tablero Kanban").unwrap(), "2026-11-15");
        assert_eq!(ultima_actividad(&d, "Otro").unwrap(), "");
    }

    #[test]
    fn orden_de_tableros_depende_de_la_semana() {
        let d = datos();
        // Semana 09-nov: ADN tiene 3600 s, Kanban 1800 s.
        assert_eq!(tableros(&d, "2026-11-09").unwrap(), ["Tablero ADN", "Tablero Kanban"]);
        // Semana 16-nov: nadie tiene horas ni concluidas; Kanban tiene 1 abierta y ADN 0.
        assert_eq!(tableros(&d, "2026-11-16").unwrap(), ["Tablero Kanban", "Tablero ADN"]);
    }

    #[test]
    fn empates_se_resuelven_por_actividad_y_nombre() {
        let d = tareas(json!([
            { "key": "X-1", "comp": "Aforo", "cat": "done", "worklogs": [] },
            { "key": "X-2", "comp": "DAI", "cat": "done", "worklogs": [] },
            { "key": "X-3", "comp": "Regímenes Especiales", "cat": "done", "worklogs": [] }
        ]));
        assert_eq!(tableros(&d, "2026-11-09").unwrap(), ["Tablero AFO", "Tablero DAI", "Tablero RE"]);
    }

    #[test]
    fn comparacion_tipo_locale() {
        assert_eq!(comparar_locale("a", "B"), Ordering::Less);
        assert_eq!(comparar_locale("Tablero AFO", "Tablero DAI"), Ordering::Less);
        assert_eq!(comparar_locale("Kanban-GPS", "Kanban GPS"), Ordering::Greater);
        assert_eq!(comparar_locale("", "2026-11-01"), Ordering::Less);
        assert_eq!(comparar_locale("2026-11-02", "2026-11-01"), Ordering::Greater);
        assert_eq!(comparar_locale("a", "A"), Ordering::Less);
        assert_eq!(comparar_locale("x", "x"), Ordering::Equal);
    }
}
