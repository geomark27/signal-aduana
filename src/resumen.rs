//! Resumen de horas para la bandeja y los avisos. Usa `domain::compute_view`:
//! no reimplementa reglas de horas ni de feriados.

use chrono::{Datelike, Duration, NaiveDate};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::domain::calendar::{fh, workdays, ExtraHolidays};
use crate::domain::view::ALL_BOARDS;
use crate::domain::{compute_view, View, ViewInput};
use crate::error::Resultado;

/// Largo máximo del texto del tooltip (Windows corta alrededor de 127 caracteres).
pub const MAX_TEXTO_BANDEJA: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resumen {
    /// `AAAA-MM-DD`
    pub hoy: String,
    pub semana_secs: i64,
    pub semana_meta_h: i64,
    pub hoy_secs: i64,
    pub hoy_laborable: bool,
    /// Semana anterior completa.
    pub anterior_secs: i64,
    pub anterior_meta_h: i64,
    /// Primer día laborable de la semana actual (`AAAA-MM-DD`); `None` si toda la semana es feriado.
    pub primer_laborable: Option<String>,
}

fn vista_de(tareas: &[Map<String, Value>], semana: NaiveDate, hoy: NaiveDate, feriados: &ExtraHolidays) -> Resultado<View> {
    compute_view(
        ViewInput {
            issues: tareas.to_vec(),
            week: semana.format("%Y-%m-%d").to_string(),
            board: ALL_BOARDS.into(),
            extra_holidays: feriados.clone(),
            today: Some(hoy.format("%Y-%m-%d").to_string()),
        },
        &hoy.format("%Y-%m-%d").to_string(),
    )
}

pub fn resumir(tareas: Vec<Map<String, Value>>, hoy: NaiveDate, feriados: &ExtraHolidays) -> Resultado<Resumen> {
    let lunes = hoy - Duration::days(i64::from(hoy.weekday().num_days_from_monday()));
    let actual = vista_de(&tareas, lunes, hoy, feriados)?;
    let anterior = vista_de(&tareas, lunes - Duration::days(7), hoy, feriados)?;
    let primer_laborable = workdays(&lunes.format("%Y-%m-%d").to_string(), feriados)?.into_iter().next();
    Ok(Resumen {
        hoy: actual.today.clone(),
        semana_secs: actual.hours.week_state.total,
        semana_meta_h: actual.hours.week_state.target_h,
        hoy_secs: actual.hours.week_state.today_secs,
        hoy_laborable: actual.hours.week_state.today_labor,
        anterior_secs: anterior.hours.week_state.total,
        anterior_meta_h: anterior.hours.week_state.target_h,
        primer_laborable,
    })
}

/// Texto del tooltip de la bandeja, p. ej. `Signal Aduana · Semana: 24 de 40 h · Hoy: 3 de 8 h`.
pub fn texto_bandeja(r: Option<&Resumen>, simulado: bool) -> String {
    let mut t = match r {
        None => "Signal Aduana · Sin datos".to_string(),
        Some(r) => {
            let semana = format!("Semana: {} de {} h", horas_sin_unidad(r.semana_secs), r.semana_meta_h);
            if r.hoy_laborable {
                format!("Signal Aduana · {semana} · Hoy: {} de 8 h", horas_sin_unidad(r.hoy_secs))
            } else {
                format!("Signal Aduana · {semana} · Hoy: no laborable")
            }
        }
    };
    if simulado {
        t.push_str(" (simulado)");
    }
    t.chars().take(MAX_TEXTO_BANDEJA).collect()
}

/// `fh` sin el sufijo « h» (`7.5 h` → `7.5`).
fn horas_sin_unidad(s: i64) -> String {
    fh(s).trim_end_matches(" h").to_string()
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::jira::simulado::JiraSimulado;
    use crate::jira::{self, Fuente};

    fn fecha(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    async fn tareas(hoy: NaiveDate) -> Vec<Map<String, Value>> {
        let f = Fuente::Simulado(JiraSimulado::nuevo("persona@ejemplo.com", true, hoy));
        let yo = f.yo().await.unwrap().account_id;
        jira::cargar_para_resumen(&f, Some(&yo), hoy).await.unwrap()
    }

    async fn contra_vista(hoy: &str, extra: &ExtraHolidays) {
        let h = fecha(hoy);
        let ts = tareas(h).await;
        let r = resumir(ts.clone(), h, extra).unwrap();
        let lunes = h - Duration::days(i64::from(h.weekday().num_days_from_monday()));
        let mk = |sem: NaiveDate| {
            compute_view(
                ViewInput {
                    issues: ts.clone(),
                    week: sem.to_string(),
                    board: "Todos".into(),
                    extra_holidays: extra.clone(),
                    today: Some(hoy.into()),
                },
                hoy,
            )
            .unwrap()
        };
        let (a, p) = (mk(lunes), mk(lunes - Duration::days(7)));
        assert_eq!(r.hoy, hoy);
        assert_eq!(r.semana_secs, a.hours.week_state.total);
        assert_eq!(r.semana_meta_h, a.hours.week_state.target_h);
        assert_eq!(r.hoy_secs, a.hours.week_state.today_secs);
        assert_eq!(r.hoy_laborable, a.hours.week_state.today_labor);
        assert_eq!(r.anterior_secs, p.hours.week_state.total);
        assert_eq!(r.anterior_meta_h, p.weeks[11].target);
        assert_eq!(r.semana_meta_h, a.weeks[11].target);
    }

    #[tokio::test]
    async fn resume_viernes_y_lunes_como_la_vista() {
        let vacio = ExtraHolidays::new();
        contra_vista("2026-11-13", &vacio).await;
        contra_vista("2026-11-16", &vacio).await;
        let mut extra = ExtraHolidays::new();
        extra.insert("2026-11-10".into(), true);
        contra_vista("2026-11-13", &extra).await;
    }

    #[tokio::test]
    async fn valores_concretos_del_simulado() {
        let r = resumir(tareas(fecha("2026-11-13")).await, fecha("2026-11-13"), &ExtraHolidays::new()).unwrap();
        assert!(r.hoy_laborable);
        assert!(r.semana_meta_h > 0 && r.anterior_meta_h > 0);
        assert!(r.semana_secs >= r.hoy_secs);
        let l = resumir(tareas(fecha("2026-11-16")).await, fecha("2026-11-16"), &ExtraHolidays::new()).unwrap();
        // El lunes, la semana anterior es la que terminó el viernes 13 (con meta completa).
        assert_eq!(l.anterior_meta_h, 40);
        assert_eq!(l.hoy_secs, 0);
    }

    #[test]
    fn fin_de_semana_no_es_laborable() {
        let r = resumir(Vec::new(), fecha("2026-11-14"), &ExtraHolidays::new()).unwrap();
        assert!(!r.hoy_laborable);
        assert_eq!(r.hoy_secs, 0);
        assert_eq!(r.semana_meta_h, 40);
    }

    fn resumen(semana: i64, hoy: i64) -> Resumen {
        Resumen {
            hoy: "2026-11-13".into(),
            semana_secs: semana,
            semana_meta_h: 40,
            hoy_secs: hoy,
            hoy_laborable: true,
            anterior_secs: 0,
            anterior_meta_h: 40,
            primer_laborable: Some("2026-11-09".into()),
        }
    }

    #[test]
    fn resumir_marca_feriado_base_y_extra() {
        let vacio = ExtraHolidays::new();
        let r = resumir(Vec::new(), fecha("2026-11-03"), &vacio).unwrap();
        assert!(!r.hoy_laborable);
        assert_eq!(r.semana_meta_h, 24);
        let mut quita = ExtraHolidays::new();
        quita.insert("2026-11-03".into(), false);
        let r = resumir(Vec::new(), fecha("2026-11-03"), &quita).unwrap();
        assert!(r.hoy_laborable);
        assert_eq!(r.semana_meta_h, 32);
        let r = resumir(Vec::new(), fecha("2026-11-14"), &vacio).unwrap();
        assert!(!r.hoy_laborable);
    }

    #[test]
    fn resumir_calcula_primer_laborable() {
        let primero = |hoy: &str, extra: &ExtraHolidays| resumir(Vec::new(), fecha(hoy), extra).unwrap().primer_laborable;
        let vacio = ExtraHolidays::new();
        assert_eq!(primero("2026-11-18", &vacio).as_deref(), Some("2026-11-16"));
        let mut lunes_feriado = ExtraHolidays::new();
        lunes_feriado.insert("2026-11-16".into(), true);
        assert_eq!(primero("2026-11-18", &lunes_feriado).as_deref(), Some("2026-11-17"));
        // El 2 y el 3 de noviembre son feriados base.
        assert_eq!(primero("2026-11-04", &vacio).as_deref(), Some("2026-11-04"));
        let toda: ExtraHolidays =
            ["2026-11-16", "2026-11-17", "2026-11-18", "2026-11-19", "2026-11-20"].iter().map(|d| (d.to_string(), true)).collect();
        assert_eq!(primero("2026-11-18", &toda), None);
    }

    #[test]
    fn texto_de_bandeja() {
        let r = resumen(24 * 3600, 3 * 3600);
        assert_eq!(texto_bandeja(Some(&r), false), "Signal Aduana · Semana: 24 de 40 h · Hoy: 3 de 8 h");
        assert_eq!(texto_bandeja(Some(&r), true), "Signal Aduana · Semana: 24 de 40 h · Hoy: 3 de 8 h (simulado)");
        assert_eq!(texto_bandeja(None, false), "Signal Aduana · Sin datos");
        assert_eq!(texto_bandeja(None, true), "Signal Aduana · Sin datos (simulado)");
        let d = resumen(27000, 5400);
        assert_eq!(texto_bandeja(Some(&d), false), "Signal Aduana · Semana: 7.5 de 40 h · Hoy: 1.5 de 8 h");
        let mut f = resumen(0, 0);
        f.hoy_laborable = false;
        assert!(texto_bandeja(Some(&f), false).ends_with("Hoy: no laborable"));
        let enorme = resumen(i64::MAX / 4, i64::MAX / 4);
        assert!(texto_bandeja(Some(&enorme), true).chars().count() <= MAX_TEXTO_BANDEJA);
    }
}
