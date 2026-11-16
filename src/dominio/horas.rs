//! Estado de las horas de una semana (`hoursState` de `ui/app.js`).
//! Los textos salen carácter por carácter como en el JS.

use serde::Serialize;

use super::calendario::{add_days, dias_laborables, es_feriado, fh, meta_semana, FeriadosExtra, META_DIA_H};
use super::semana::{estadisticas_semana, segundos_del_dia};
use super::tarea::TareaDerivada;
use crate::error::Resultado;

/// Los campos que devuelve `hoursState(wk)` (sin los extras de la vista).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EstadoSemana {
    pub total: i64,
    pub target: i64,
    pub target_h: i64,
    pub expected: i64,
    pub missing: i64,
    pub behind: i64,
    pub sev: String,
    pub title: String,
    pub detail: String,
    pub current: bool,
    pub today_secs: i64,
    pub today_labor: bool,
    pub today: String,
}

fn dias(n: usize) -> &'static str {
    if n == 1 { "día" } else { "días" }
}

pub fn estado_horas(tareas: &[TareaDerivada], wk: &str, hoy: &str, extra: &FeriadosExtra) -> Resultado<EstadoSemana> {
    let total = estadisticas_semana(tareas, wk, &|_| true)?.secs;
    let target_h = meta_semana(wk, extra)?;
    let target = target_h * 3600;
    let fin = add_days(wk, 4)?;
    let current = hoy >= wk && hoy <= fin.as_str();
    let future = hoy < wk;
    let worked_days = dias_laborables(wk, extra)?.iter().filter(|d| d.as_str() < hoy).count();
    let today_secs = if current { segundos_del_dia(tareas, hoy) } else { 0 };
    let today_labor = current && !es_feriado(hoy, extra) && hoy <= fin.as_str();
    let dia_seg = META_DIA_H * 3600;
    let expected = if current { worked_days as i64 * dia_seg } else { target };
    let missing = (target - total).max(0);
    let behind = (expected - total).max(0);

    let (sev, title, detail): (&str, String, String);
    if target_h == 0 {
        sev = "";
        title = "Semana de feriados".into();
        detail = format!("No hay días laborables en esta semana.{}", if total != 0 { format!(" Registraste {}.", fh(total)) } else { String::new() });
    } else if future {
        sev = "";
        title = "Semana futura".into();
        detail = "Todavía no hay horas para esta semana.".into();
    } else if total >= target {
        sev = "ok";
        title = if total > target { format!("Semana completa: {} registradas (+{})", fh(total), fh(total - target)) } else { "Semana completa".into() };
        detail = if current { "Ya cumpliste las horas de la semana.".into() } else { "Cumpliste las horas de esa semana.".into() };
    } else if !current {
        sev = "crit";
        title = format!("Faltaron {} para completar las {} h", fh(missing), target_h);
        detail = "Esa semana ya cerró. Si trabajaste esas horas, regístralas en Jira con su fecha real.".into();
    } else if behind >= dia_seg {
        sev = "crit";
        title = format!("Te faltan {} para las {} h", fh(missing), target_h);
        detail = format!(
            "Vas {} por debajo de lo esperado hasta ayer ({} en {} {} laborables). Revisa si te falta registrar horas.",
            fh(behind), fh(expected), worked_days, dias(worked_days)
        );
    } else if behind > 0 {
        sev = "warn";
        title = format!("Te faltan {} para las {} h", fh(missing), target_h);
        detail = format!("Vas {} por debajo de lo esperado hasta ayer ({}).", fh(behind), fh(expected));
    } else {
        title = format!("Te faltan {} para las {} h", fh(missing), target_h);
        if worked_days > 0 {
            sev = "warn";
            detail = format!("Vas al día: lo esperado hasta ayer era {}.", fh(expected));
        } else {
            sev = "";
            detail = "Empieza la semana: registra tus horas cada día.".into();
        }
    }
    Ok(EstadoSemana {
        total, target, target_h, expected, missing, behind,
        sev: sev.into(), title, detail, current, today_secs, today_labor, today: hoy.into(),
    })
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::dominio::tarea::derivar;
    use serde_json::{json, Value};

    /// Tareas con un registro por día laborable indicado, de `h` horas cada uno.
    fn con_horas(dias: &[(&str, i64)]) -> Vec<TareaDerivada> {
        let wls: Vec<Value> = dias.iter().map(|(d, s)| json!({ "d": format!("{d}T12:00:00.000-05:00"), "s": s })).collect();
        let t = json!({ "key": "A-1", "worklogs": wls });
        derivar(&[t.as_object().unwrap().clone()]).unwrap()
    }

    fn est(t: &[TareaDerivada], wk: &str, hoy: &str) -> EstadoSemana {
        estado_horas(t, wk, hoy, &FeriadosExtra::new()).unwrap()
    }

    const H8: i64 = 28800;

    #[test]
    fn semana_futura() {
        let e = est(&[], "2026-11-16", "2026-11-13");
        assert_eq!((e.sev.as_str(), e.title.as_str()), ("", "Semana futura"));
        assert_eq!(e.detail, "Todavía no hay horas para esta semana.");
        assert!(!e.current);
        assert_eq!(e.expected, 40 * 3600);
    }

    #[test]
    fn semana_pasada_completa_con_exceso_e_incompleta() {
        let c = con_horas(&[("2026-11-09", H8), ("2026-11-10", H8), ("2026-11-11", H8), ("2026-11-12", H8), ("2026-11-13", H8)]);
        let e = est(&c, "2026-11-09", "2026-11-18");
        assert_eq!((e.sev.as_str(), e.title.as_str()), ("ok", "Semana completa"));
        assert_eq!(e.detail, "Cumpliste las horas de esa semana.");

        let c = con_horas(&[("2026-11-09", 3600 * 41)]);
        let e = est(&c, "2026-11-09", "2026-11-18");
        assert_eq!(e.title, "Semana completa: 41 h registradas (+1 h)");

        let c = con_horas(&[("2026-11-09", 27000)]);
        let e = est(&c, "2026-11-09", "2026-11-18");
        assert_eq!(e.sev, "crit");
        assert_eq!(e.title, "Faltaron 32.5 h para completar las 40 h");
        assert!(e.detail.starts_with("Esa semana ya cerró."));
        assert_eq!(e.missing, 40 * 3600 - 27000);
    }

    #[test]
    fn semana_actual_atrasada() {
        // Viernes: 4 días laborables transcurridos, esperado 32 h; hay 8 h.
        let c = con_horas(&[("2026-11-09", H8)]);
        let e = est(&c, "2026-11-09", "2026-11-13");
        assert!(e.current);
        assert_eq!(e.expected, 4 * H8);
        assert_eq!(e.behind, 3 * H8);
        assert_eq!(e.sev, "crit");
        assert_eq!(e.title, "Te faltan 32 h para las 40 h");
        assert_eq!(e.detail, "Vas 24 h por debajo de lo esperado hasta ayer (32 h en 4 días laborables). Revisa si te falta registrar horas.");
    }

    #[test]
    fn semana_actual_poco_por_debajo_y_al_dia() {
        // Jueves: esperado 24 h (3 días). Con 20 h: por debajo menos de un día.
        let c = con_horas(&[("2026-11-09", 3600 * 20)]);
        let e = est(&c, "2026-11-09", "2026-11-12");
        assert_eq!(e.sev, "warn");
        assert_eq!(e.behind, 4 * 3600);
        assert_eq!(e.detail, "Vas 4 h por debajo de lo esperado hasta ayer (24 h).");

        let c = con_horas(&[("2026-11-09", 3600 * 24)]);
        let e = est(&c, "2026-11-09", "2026-11-12");
        assert_eq!((e.sev.as_str(), e.behind), ("warn", 0));
        assert_eq!(e.title, "Te faltan 16 h para las 40 h");
        assert_eq!(e.detail, "Vas al día: lo esperado hasta ayer era 24 h.");
    }

    #[test]
    fn primer_dia_de_la_semana() {
        let e = est(&[], "2026-11-09", "2026-11-09");
        assert_eq!(e.sev, "");
        assert_eq!(e.detail, "Empieza la semana: registra tus horas cada día.");
        assert_eq!(e.title, "Te faltan 40 h para las 40 h");
        assert!(e.today_labor);
        assert_eq!(e.expected, 0);
    }

    #[test]
    fn semana_de_feriados_y_dia_de_hoy() {
        let mut extra = FeriadosExtra::new();
        for d in ["2026-11-09", "2026-11-10", "2026-11-11", "2026-11-12", "2026-11-13"] {
            extra.insert(d.into(), true);
        }
        let c = con_horas(&[("2026-11-09", 5400)]);
        let e = estado_horas(&c, "2026-11-09", "2026-11-10", &extra).unwrap();
        assert_eq!(e.title, "Semana de feriados");
        assert_eq!(e.detail, "No hay días laborables en esta semana. Registraste 1.5 h.");
        assert!(!e.today_labor);
        let e = estado_horas(&[], "2026-11-09", "2026-11-10", &extra).unwrap();
        assert_eq!(e.detail, "No hay días laborables en esta semana.");
        // Un solo día laborable: el singular.
        let mut extra = FeriadosExtra::new();
        for d in ["2026-11-10", "2026-11-11", "2026-11-12", "2026-11-13"] {
            extra.insert(d.into(), true);
        }
        let e = estado_horas(&[], "2026-11-09", "2026-11-12", &extra).unwrap();
        assert_eq!(e.sev, "crit");
        assert!(e.detail.contains("(8 h en 1 día laborables)"), "{}", e.detail);
    }

    #[test]
    fn horas_de_hoy_cuentan_solo_en_la_semana_actual() {
        let c = con_horas(&[("2026-11-12", 5400)]);
        assert_eq!(est(&c, "2026-11-09", "2026-11-12").today_secs, 5400);
        assert_eq!(est(&c, "2026-11-02", "2026-11-12").today_secs, 0);
        // Sábado de la semana actual: no es `current` (hoy > viernes).
        assert!(!est(&c, "2026-11-09", "2026-11-14").current);
    }
}
