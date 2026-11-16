//! Arma la `Vista` completa (Contrato D) a partir de la `Entrada`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::calendario::{
    add_days, dias_laborables, es_feriado, es_lunes, feriados_semana, iso_week, meta_semana, validar_fecha, FeriadosExtra,
};
use super::horas::{estado_horas, EstadoSemana};
use super::semana::{estadisticas_semana, segundos_del_dia, tableros, ultima_actividad, Stats};
use super::tarea::{derivar, TareaDerivada};
use crate::error::{ErrorApp, Resultado};

pub const TODOS: &str = "Todos";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entrada {
    pub tareas: Vec<Map<String, Value>>,
    pub semana: String,
    pub tablero: String,
    #[serde(default)]
    pub feriados_extra: FeriadosExtra,
    #[serde(default)]
    pub hoy: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EstadoHoras {
    #[serde(flatten)]
    pub estado: EstadoSemana,
    pub prev_secs: i64,
    pub laborables: i64,
    pub dias: Vec<DiaHoras>,
    pub feriados: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DiaHoras {
    pub day: String,
    pub secs: i64,
    pub feriado: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SemanaGrafico {
    pub wk: String,
    pub semana_iso: i64,
    pub secs: i64,
    pub per_board: BTreeMap<String, i64>,
    /// Meta de la semana, en horas.
    pub target: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FilaTablero {
    pub board: String,
    pub secs: i64,
    pub worked: i64,
    pub resolved: i64,
    pub abiertas: i64,
    pub estados: BTreeMap<String, i64>,
    pub ultima: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Vista {
    pub hoy: String,
    pub semana: String,
    pub tablero: String,
    pub tareas: Vec<TareaDerivada>,
    pub tableros: Vec<String>,
    pub stats: Stats,
    pub horas: EstadoHoras,
    pub semanas: Vec<SemanaGrafico>,
    #[serde(rename = "porTablero")]
    pub por_tablero: Vec<FilaTablero>,
}

/// Calcula la vista. `hoy_por_defecto` se usa cuando la entrada no trae `hoy`.
pub fn calcular_vista(entrada: Entrada, hoy_por_defecto: &str) -> Resultado<Vista> {
    let semana = validar_fecha(&entrada.semana)?;
    if !es_lunes(&semana)? {
        return Err(ErrorApp::Formato(format!("La semana debe empezar en lunes: «{semana}»")));
    }
    let hoy = validar_fecha(entrada.hoy.as_deref().unwrap_or(hoy_por_defecto))?;
    let extra = &entrada.feriados_extra;

    let tareas = derivar(&entrada.tareas)?;
    let nombres = tableros(&tareas, &semana)?;
    let tablero = if entrada.tablero == TODOS || nombres.contains(&entrada.tablero) { entrada.tablero.clone() } else { TODOS.into() };
    let en_tablero = |t: &TareaDerivada| tablero == TODOS || t.c.board == tablero;

    let stats = estadisticas_semana(&tareas, &semana, &en_tablero)?;

    let estado = estado_horas(&tareas, &semana, &hoy, extra)?;
    let prev_secs = estadisticas_semana(&tareas, &add_days(&semana, -7)?, &|_| true)?.secs;
    let mut dias = Vec::new();
    for k in 0..5 {
        let day = add_days(&semana, k)?;
        dias.push(DiaHoras { secs: segundos_del_dia(&tareas, &day), feriado: es_feriado(&day, extra), day });
    }
    let horas = EstadoHoras {
        estado,
        prev_secs,
        laborables: dias_laborables(&semana, extra)?.len() as i64,
        dias,
        feriados: feriados_semana(&semana, extra)?,
    };

    let mut semanas = Vec::new();
    for k in (0..=11).rev() {
        let wk = add_days(&semana, -7 * k)?;
        let s = estadisticas_semana(&tareas, &wk, &en_tablero)?;
        semanas.push(SemanaGrafico { semana_iso: iso_week(&wk)?.1, secs: s.secs, per_board: s.per_board, target: meta_semana(&wk, extra)?, wk });
    }

    let mut por_tablero = Vec::new();
    for b in &nombres {
        let s = estadisticas_semana(&tareas, &semana, &|t| t.c.board == *b)?;
        let mut estados = BTreeMap::new();
        let mut abiertas = 0;
        for t in tareas.iter().filter(|t| t.c.board == *b && t.c.cat.as_deref() != Some("done")) {
            abiertas += 1;
            // `st[i.status]` en JS: un estado ausente se convierte en el texto «null».
            *estados.entry(t.c.status.clone().unwrap_or_else(|| "null".into())).or_insert(0) += 1;
        }
        por_tablero.push(FilaTablero {
            board: b.clone(),
            secs: s.secs,
            worked: s.worked.len() as i64,
            resolved: s.resolved.len() as i64,
            abiertas,
            estados,
            ultima: ultima_actividad(&tareas, b)?,
        });
    }

    Ok(Vista { hoy, semana, tablero, tareas, tableros: nombres, stats, horas, semanas, por_tablero })
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use serde_json::json;

    fn entrada(semana: &str, tablero: &str, hoy: Option<&str>) -> Entrada {
        let tareas = json!([
            { "key": "A-1", "project": "ADN", "status": "En curso", "cat": "indeterminate", "worklogs": [{ "d": "2026-11-10T10:00:00.000-05:00", "s": 7200 }] },
            { "key": "A-2", "project": "ADN", "status": null, "cat": "new", "worklogs": [] },
            { "key": "G-1", "project": "GPS", "type": "Tarea", "status": "Hecho", "cat": "done", "finDev": "2026-11-11", "worklogs": [] }
        ]);
        Entrada {
            tareas: tareas.as_array().unwrap().iter().map(|x| x.as_object().unwrap().clone()).collect(),
            semana: semana.into(),
            tablero: tablero.into(),
            feriados_extra: FeriadosExtra::new(),
            hoy: hoy.map(String::from),
        }
    }

    #[test]
    fn vista_completa() {
        let v = calcular_vista(entrada("2026-11-09", "Todos", Some("2026-11-13")), "2000-01-01").unwrap();
        assert_eq!(v.hoy, "2026-11-13");
        assert_eq!(v.tablero, "Todos");
        assert_eq!(v.tableros, ["Tablero ADN", "Tablero Kanban"]);
        assert_eq!(v.stats.secs, 7200);
        assert_eq!(v.semanas.len(), 12);
        assert_eq!(v.semanas[11].wk, "2026-11-09");
        assert_eq!(v.semanas[0].wk, "2026-08-24");
        assert_eq!(v.semanas[11].semana_iso, 46);
        assert_eq!(v.semanas[11].target, 40);
        assert_eq!(v.horas.dias.len(), 5);
        assert_eq!(v.horas.dias[1].secs, 7200);
        assert_eq!(v.horas.prev_secs, 0);
        assert_eq!(v.horas.laborables, 5);
        assert_eq!(v.por_tablero.len(), 2);
        assert_eq!(v.por_tablero[0].board, "Tablero ADN");
        assert_eq!(v.por_tablero[0].abiertas, 2);
        assert_eq!(v.por_tablero[0].estados["En curso"], 1);
        assert_eq!(v.por_tablero[0].estados["null"], 1);
        let j = serde_json::to_value(&v).unwrap();
        assert_eq!(j["horas"]["targetH"], 40);
        assert_eq!(j["horas"]["prevSecs"], 0);
        assert_eq!(j["semanas"][0]["semanaIso"], 35);
        assert_eq!(j["tareas"][0]["board"], "Tablero ADN");
    }

    #[test]
    fn tablero_filtra_y_el_inexistente_vuelve_a_todos() {
        let v = calcular_vista(entrada("2026-11-09", "Tablero Kanban", Some("2026-11-13")), "").unwrap();
        assert_eq!(v.tablero, "Tablero Kanban");
        assert_eq!(v.stats.secs, 0);
        assert_eq!(v.stats.resolved, ["G-1"]);
        // `horas` y `porTablero` no filtran.
        assert_eq!(v.horas.estado.total, 7200);
        assert_eq!(v.por_tablero.len(), 2);
        let v = calcular_vista(entrada("2026-11-09", "No existe", Some("2026-11-13")), "").unwrap();
        assert_eq!(v.tablero, "Todos");
        assert_eq!(v.stats.secs, 7200);
    }

    #[test]
    fn semana_invalida_es_error_de_formato() {
        let e = calcular_vista(entrada("2026-11-10", "Todos", None), "2026-11-13").unwrap_err();
        assert_eq!(e.codigo(), "formato");
        let e = calcular_vista(entrada("hoy", "Todos", None), "2026-11-13").unwrap_err();
        assert_eq!(e.codigo(), "formato");
        let e = calcular_vista(entrada("2026-11-09", "Todos", Some("mañana")), "2026-11-13").unwrap_err();
        assert_eq!(e.codigo(), "formato");
    }

    #[test]
    fn hoy_por_defecto_y_orden_de_tareas() {
        let v = calcular_vista(entrada("2026-11-09", "Todos", None), "2026-11-12").unwrap();
        assert_eq!(v.hoy, "2026-11-12");
        let claves: Vec<_> = v.tareas.iter().map(|t| t.c.key.as_str()).collect();
        assert_eq!(claves, ["A-1", "A-2", "G-1"]);
    }
}
