//! Fechas, semanas, feriados, metas y formato de horas.
//!
//! Port literal de `localDate`, `dateOnly`, `mondayOf`, `addDays`, `isoWeek`, `fmtD`, `h`, `fh`,
//! `isFeriado`, `laborDays`, `weekTarget` y `weekHolidays` de `ui/app.js`. Las fechas viajan como
//! texto `AAAA-MM-DD` (se comparan como texto, igual que en JS) y todo se calcula en UTC.

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime};

use crate::error::{ErrorApp, Resultado};

/// Horas de la meta semanal completa y de una jornada.
pub const META_SEMANA_H: i64 = 40;
pub const META_DIA_H: i64 = 8;
/// Desfase de Ecuador respecto a UTC, en horas (sin horario de verano).
pub const DESFASE_HORAS: i64 = -5;

/// Feriados nacionales base (los mismos de `FERIADOS_BASE`).
pub const FERIADOS_BASE: [&str; 12] = [
    "2026-01-01", "2026-02-16", "2026-02-17", "2026-04-03", "2026-05-01", "2026-05-25",
    "2026-08-10", "2026-10-09", "2026-11-02", "2026-11-03", "2026-12-25", "2027-01-01",
];

const MES: [&str; 12] = ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic"];
pub const DIA: [&str; 7] = ["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"];

/// Cambios del usuario sobre los feriados base (`feriadosExtra`).
pub type FeriadosExtra = BTreeMap<String, bool>;

fn fecha(ymd: &str) -> Resultado<NaiveDate> {
    if ymd.len() != 10 {
        return Err(ErrorApp::Formato(format!("Fecha no válida: «{ymd}»")));
    }
    NaiveDate::parse_from_str(ymd, "%Y-%m-%d").map_err(|_| ErrorApp::Formato(format!("Fecha no válida: «{ymd}»")))
}

fn texto(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Interpreta un texto de fecha como lo hace `new Date(s)` en las formas que usa la app:
/// RFC 3339, fecha y hora con desfase `+hhmm`, fecha-hora sin desfase y fecha sola
/// (estos dos últimos se toman como UTC, igual que `new Date("2026-11-10")`).
/// Devuelve milisegundos desde la época.
pub fn instante_ms(s: &str) -> Option<i64> {
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(d.timestamp_millis());
    }
    if let Ok(d) = DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f%z") {
        return Some(d.timestamp_millis());
    }
    if let Ok(d) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(d.and_utc().timestamp_millis());
    }
    if s.len() == 10 {
        return NaiveDate::parse_from_str(s, "%Y-%m-%d").ok().map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp_millis());
    }
    None
}

/// `localDate`: suma el desfase de Ecuador al instante UTC y corta la fecha.
pub fn local_date(iso: &str) -> Resultado<String> {
    let ms = instante_ms(iso).ok_or_else(|| ErrorApp::Formato(format!("Fecha-hora no válida: «{iso}»")))?;
    let d = DateTime::from_timestamp_millis(ms + DESFASE_HORAS * 3_600_000)
        .ok_or_else(|| ErrorApp::Formato(format!("Fecha-hora fuera de rango: «{iso}»")))?;
    Ok(texto(d.date_naive()))
}

/// `dateOnly`: vacío → `None`; más de 10 caracteres → `local_date`; si no, tal cual.
pub fn date_only(s: Option<&str>) -> Resultado<Option<String>> {
    match s {
        None | Some("") => Ok(None),
        Some(s) if s.chars().count() > 10 => local_date(s).map(Some),
        Some(s) => Ok(Some(s.to_string())),
    }
}

/// Día de la semana con lunes = 0.
pub fn dia_semana(ymd: &str) -> Resultado<u32> {
    Ok(fecha(ymd)?.weekday().num_days_from_monday())
}

pub fn monday_of(ymd: &str) -> Resultado<String> {
    let d = fecha(ymd)?;
    Ok(texto(d - Duration::days(d.weekday().num_days_from_monday() as i64)))
}

pub fn add_days(ymd: &str, n: i64) -> Resultado<String> {
    Ok(texto(fecha(ymd)? + Duration::days(n)))
}

/// ¿La fecha `AAAA-MM-DD` es lunes?
pub fn es_lunes(ymd: &str) -> Resultado<bool> {
    Ok(dia_semana(ymd)? == 0)
}

/// `isoWeek`: (año ISO, número de semana ISO).
pub fn iso_week(ymd: &str) -> Resultado<(i32, i64)> {
    let d = fecha(ymd)?;
    let wd = d.weekday().num_days_from_monday() as i64;
    let th = d + Duration::days(3 - wd);
    let y = th.year();
    let j1 = NaiveDate::from_ymd_opt(y, 1, 4).ok_or_else(|| ErrorApp::Formato(format!("Año fuera de rango: {y}")))?;
    let wj1 = j1.weekday().num_days_from_monday() as i64;
    let dias = (th - j1).num_days() - 3 + wj1;
    // `Math.round(dias / 7)`: `dias` es múltiplo de 7, pero se mantiene el redondeo de JS.
    Ok((y, 1 + redondeo_js(dias as f64 / 7.0) as i64))
}

/// `Math.round` de JS: la mitad redondea hacia +∞.
pub fn redondeo_js(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// `fmtD`: `13 nov`, o `—` si no hay fecha.
pub fn fmt_d(ymd: Option<&str>) -> String {
    let Some(s) = ymd.filter(|s| !s.is_empty()) else { return "—".into() };
    let mut p = s.split('-');
    let (_, m, d) = (p.next(), p.next().unwrap_or(""), p.next().unwrap_or(""));
    let (m, d): (i64, i64) = (m.parse().unwrap_or(0), d.parse().unwrap_or(0));
    let mes = if (1..=12).contains(&m) { MES[(m - 1) as usize] } else { "undefined" };
    format!("{d} {mes}")
}

/// `h(s)`: segundos → décimas de hora (`Math.round(s/360)`). La hora es esto / 10.
pub fn decimas(s: i64) -> i64 {
    redondeo_js(s as f64 / 360.0) as i64
}

/// `fh(s)`: `7.5 h` u `8 h`.
pub fn fh(s: i64) -> String {
    let t = decimas(s);
    if t % 10 == 0 {
        format!("{} h", t / 10)
    } else {
        let signo = if t < 0 { "-" } else { "" };
        format!("{signo}{}.{} h", t.abs() / 10, t.abs() % 10)
    }
}

/// `isFeriado`: el cambio del usuario manda sobre los feriados base.
pub fn es_feriado(ymd: &str, extra: &FeriadosExtra) -> bool {
    match extra.get(ymd) {
        Some(v) => *v,
        None => FERIADOS_BASE.contains(&ymd),
    }
}

/// `laborDays`: lunes a viernes que no son feriado.
pub fn dias_laborables(wk: &str, extra: &FeriadosExtra) -> Resultado<Vec<String>> {
    let mut out = Vec::new();
    for d in 0..5 {
        let day = add_days(wk, d)?;
        if !es_feriado(&day, extra) {
            out.push(day);
        }
    }
    Ok(out)
}

/// `weekHolidays`: lunes a viernes que son feriado.
pub fn feriados_semana(wk: &str, extra: &FeriadosExtra) -> Resultado<Vec<String>> {
    let mut out = Vec::new();
    for d in 0..5 {
        let day = add_days(wk, d)?;
        if es_feriado(&day, extra) {
            out.push(day);
        }
    }
    Ok(out)
}

/// `weekTarget`: meta de horas de la semana (8 h por día laborable).
pub fn meta_semana(wk: &str, extra: &FeriadosExtra) -> Resultado<i64> {
    Ok(dias_laborables(wk, extra)?.len() as i64 * META_DIA_H)
}

/// Valida `AAAA-MM-DD` y lo devuelve normalizado.
pub fn validar_fecha(ymd: &str) -> Resultado<String> {
    fecha(ymd).map(texto)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn sin_extra() -> FeriadosExtra {
        FeriadosExtra::new()
    }

    #[test]
    fn local_date_resta_cinco_horas() {
        assert_eq!(local_date("2026-11-10T08:30:00.000-05:00").unwrap(), "2026-11-10");
        // 02:00 UTC es la noche anterior en Ecuador.
        assert_eq!(local_date("2026-11-10T02:00:00.000Z").unwrap(), "2026-11-09");
        assert_eq!(local_date("2026-11-10T04:59:59.999Z").unwrap(), "2026-11-09");
        assert_eq!(local_date("2026-11-10T05:00:00.000Z").unwrap(), "2026-11-10");
        // Cambio de año.
        assert_eq!(local_date("2027-01-01T03:00:00.000Z").unwrap(), "2026-12-31");
        // Una fecha sola es medianoche UTC: en Ecuador cae el día anterior (igual que en JS).
        assert_eq!(local_date("2026-11-10").unwrap(), "2026-11-09");
        assert!(local_date("no es fecha").is_err());
    }

    #[test]
    fn date_only_respeta_el_largo() {
        assert_eq!(date_only(None).unwrap(), None);
        assert_eq!(date_only(Some("")).unwrap(), None);
        assert_eq!(date_only(Some("2026-11-10")).unwrap().as_deref(), Some("2026-11-10"));
        assert_eq!(date_only(Some("2026-11-10T01:00:00.000+00:00")).unwrap().as_deref(), Some("2026-11-09"));
    }

    #[test]
    fn lunes_y_suma_de_dias() {
        assert_eq!(monday_of("2026-11-15").unwrap(), "2026-11-09"); // domingo
        assert_eq!(monday_of("2026-11-09").unwrap(), "2026-11-09");
        assert_eq!(monday_of("2027-01-01").unwrap(), "2026-12-28");
        assert_eq!(add_days("2026-12-30", 3).unwrap(), "2027-01-02");
        assert_eq!(add_days("2026-03-01", -1).unwrap(), "2026-02-28");
        assert!(add_days("2026-13-01", 1).is_err());
        assert!(es_lunes("2026-11-09").unwrap());
        assert!(!es_lunes("2026-11-10").unwrap());
    }

    #[test]
    fn semana_iso_con_cambio_de_anio() {
        assert_eq!(iso_week("2026-11-09").unwrap(), (2026, 46));
        assert_eq!(iso_week("2026-12-28").unwrap(), (2026, 53));
        assert_eq!(iso_week("2027-01-01").unwrap(), (2026, 53));
        assert_eq!(iso_week("2027-01-04").unwrap(), (2027, 1));
        assert_eq!(iso_week("2024-12-30").unwrap(), (2025, 1));
        assert_eq!(iso_week("2026-01-01").unwrap(), (2026, 1));
    }

    #[test]
    fn formato_de_fechas_y_horas() {
        assert_eq!(fmt_d(Some("2026-11-03")), "3 nov");
        assert_eq!(fmt_d(Some("2026-12-25")), "25 dic");
        assert_eq!(fmt_d(None), "—");
        assert_eq!(fh(27000), "7.5 h");
        assert_eq!(fh(28800), "8 h");
        assert_eq!(fh(0), "0 h");
        assert_eq!(fh(3600 * 40 + 1800), "40.5 h");
        // 54 s = 0,15 décimas de 360 s → 0; 180 s = 0,5 → 1 (la mitad sube).
        assert_eq!(decimas(179), 0);
        assert_eq!(decimas(180), 1);
        assert_eq!(fh(180), "0.1 h");
        assert_eq!(redondeo_js(-2.5), -2.0);
        assert_eq!(redondeo_js(2.5), 3.0);
    }

    #[test]
    fn feriados_base_quitados_y_agregados() {
        let mut extra = sin_extra();
        assert!(es_feriado("2026-11-02", &extra));
        assert!(!es_feriado("2026-11-04", &extra));
        assert_eq!(feriados_semana("2026-11-02", &extra).unwrap(), ["2026-11-02", "2026-11-03"]);
        assert_eq!(meta_semana("2026-11-02", &extra).unwrap(), 24);
        extra.insert("2026-11-02".into(), false); // quita uno base
        extra.insert("2026-11-04".into(), true); // agrega uno
        assert!(!es_feriado("2026-11-02", &extra));
        assert!(es_feriado("2026-11-04", &extra));
        assert_eq!(meta_semana("2026-11-02", &extra).unwrap(), 24);
        assert_eq!(dias_laborables("2026-11-02", &extra).unwrap(), ["2026-11-02", "2026-11-05", "2026-11-06"]);
        assert_eq!(meta_semana("2026-11-09", &sin_extra()).unwrap(), 40);
    }
}
