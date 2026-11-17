//! Dates, weeks, holidays, targets and hour formatting.
//!
//! Literal port of `localDate`, `dateOnly`, `mondayOf`, `addDays`, `isoWeek`, `fmtD`, `h`, `fh`,
//! `isFeriado`, `laborDays`, `weekTarget` and `weekHolidays` from `ui/app.js`. Dates travel as
//! `YYYY-MM-DD` text (compared as text, like in JS) and everything is computed in UTC.

use std::collections::BTreeMap;

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime};

use crate::error::{AppError, AppResult};

/// Hours of the full weekly target and of one workday.
pub const WEEK_TARGET_H: i64 = 40;
pub const DAY_TARGET_H: i64 = 8;
/// Ecuador's offset from UTC, in hours (no daylight saving time).
pub const UTC_OFFSET_HOURS: i64 = -5;

/// Base national holidays (the same as `FERIADOS_BASE`).
pub const BASE_HOLIDAYS: [&str; 12] = [
    "2026-01-01", "2026-02-16", "2026-02-17", "2026-04-03", "2026-05-01", "2026-05-25",
    "2026-08-10", "2026-10-09", "2026-11-02", "2026-11-03", "2026-12-25", "2027-01-01",
];

const MONTHS: [&str; 12] = ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic"];
pub const WEEKDAYS: [&str; 7] = ["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"];

/// User changes on top of the base holidays (`feriadosExtra`).
pub type ExtraHolidays = BTreeMap<String, bool>;

fn parse_date(ymd: &str) -> AppResult<NaiveDate> {
    if ymd.len() != 10 {
        return Err(AppError::Format(format!("Fecha no válida: «{ymd}»")));
    }
    NaiveDate::parse_from_str(ymd, "%Y-%m-%d").map_err(|_| AppError::Format(format!("Fecha no válida: «{ymd}»")))
}

fn date_text(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Interprets a date text the way `new Date(s)` does in the forms the app uses:
/// RFC 3339, date-time with a `+hhmm` offset, date-time without offset and bare date
/// (the last two are taken as UTC, like `new Date("2026-11-10")`).
/// Returns milliseconds since the epoch.
pub fn instant_ms(s: &str) -> Option<i64> {
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

/// `localDate`: adds Ecuador's offset to the UTC instant and cuts the date.
pub fn local_date(iso: &str) -> AppResult<String> {
    let ms = instant_ms(iso).ok_or_else(|| AppError::Format(format!("Fecha-hora no válida: «{iso}»")))?;
    let d = DateTime::from_timestamp_millis(ms + UTC_OFFSET_HOURS * 3_600_000)
        .ok_or_else(|| AppError::Format(format!("Fecha-hora fuera de rango: «{iso}»")))?;
    Ok(date_text(d.date_naive()))
}

/// `dateOnly`: empty → `None`; longer than 10 characters → `local_date`; otherwise as is.
pub fn date_only(s: Option<&str>) -> AppResult<Option<String>> {
    match s {
        None | Some("") => Ok(None),
        Some(s) if s.chars().count() > 10 => local_date(s).map(Some),
        Some(s) => Ok(Some(s.to_string())),
    }
}

/// Day of the week with Monday = 0.
pub fn weekday_index(ymd: &str) -> AppResult<u32> {
    Ok(parse_date(ymd)?.weekday().num_days_from_monday())
}

pub fn monday_of(ymd: &str) -> AppResult<String> {
    let d = parse_date(ymd)?;
    Ok(date_text(d - Duration::days(d.weekday().num_days_from_monday() as i64)))
}

pub fn add_days(ymd: &str, n: i64) -> AppResult<String> {
    Ok(date_text(parse_date(ymd)? + Duration::days(n)))
}

/// Is the `YYYY-MM-DD` date a Monday?
pub fn is_monday(ymd: &str) -> AppResult<bool> {
    Ok(weekday_index(ymd)? == 0)
}

/// `isoWeek`: (ISO year, ISO week number).
pub fn iso_week(ymd: &str) -> AppResult<(i32, i64)> {
    let d = parse_date(ymd)?;
    let wd = d.weekday().num_days_from_monday() as i64;
    let th = d + Duration::days(3 - wd);
    let y = th.year();
    let j1 = NaiveDate::from_ymd_opt(y, 1, 4).ok_or_else(|| AppError::Format(format!("Año fuera de rango: {y}")))?;
    let wj1 = j1.weekday().num_days_from_monday() as i64;
    let days = (th - j1).num_days() - 3 + wj1;
    // `Math.round(days / 7)`: `days` is a multiple of 7, but JS rounding is kept.
    Ok((y, 1 + js_round(days as f64 / 7.0) as i64))
}

/// JS `Math.round`: halves round toward +∞.
pub fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// `fmtD`: `13 nov`, or `—` when there is no date.
pub fn fmt_d(ymd: Option<&str>) -> String {
    let Some(s) = ymd.filter(|s| !s.is_empty()) else { return "—".into() };
    let mut p = s.split('-');
    let (_, m, d) = (p.next(), p.next().unwrap_or(""), p.next().unwrap_or(""));
    let (m, d): (i64, i64) = (m.parse().unwrap_or(0), d.parse().unwrap_or(0));
    let month = if (1..=12).contains(&m) { MONTHS[(m - 1) as usize] } else { "undefined" };
    format!("{d} {month}")
}

/// `h(s)`: seconds → tenths of an hour (`Math.round(s/360)`). The hour is this / 10.
pub fn tenths(s: i64) -> i64 {
    js_round(s as f64 / 360.0) as i64
}

/// `fh(s)`: `7.5 h` or `8 h`.
pub fn fh(s: i64) -> String {
    let t = tenths(s);
    if t % 10 == 0 {
        format!("{} h", t / 10)
    } else {
        let sign = if t < 0 { "-" } else { "" };
        format!("{sign}{}.{} h", t.abs() / 10, t.abs() % 10)
    }
}

/// `isFeriado`: the user's change wins over the base holidays.
pub fn is_holiday(ymd: &str, extra: &ExtraHolidays) -> bool {
    match extra.get(ymd) {
        Some(v) => *v,
        None => BASE_HOLIDAYS.contains(&ymd),
    }
}

/// `laborDays`: Monday to Friday that are not holidays.
pub fn workdays(wk: &str, extra: &ExtraHolidays) -> AppResult<Vec<String>> {
    let mut out = Vec::new();
    for d in 0..5 {
        let day = add_days(wk, d)?;
        if !is_holiday(&day, extra) {
            out.push(day);
        }
    }
    Ok(out)
}

/// `weekHolidays`: Monday to Friday that are holidays.
pub fn week_holidays(wk: &str, extra: &ExtraHolidays) -> AppResult<Vec<String>> {
    let mut out = Vec::new();
    for d in 0..5 {
        let day = add_days(wk, d)?;
        if is_holiday(&day, extra) {
            out.push(day);
        }
    }
    Ok(out)
}

/// `weekTarget`: the week's hour target (8 h per workday).
pub fn week_target(wk: &str, extra: &ExtraHolidays) -> AppResult<i64> {
    Ok(workdays(wk, extra)?.len() as i64 * DAY_TARGET_H)
}

/// Validates `YYYY-MM-DD` and returns it normalized.
pub fn validate_date(ymd: &str) -> AppResult<String> {
    parse_date(ymd).map(date_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_extra() -> ExtraHolidays {
        ExtraHolidays::new()
    }

    #[test]
    fn local_date_subtracts_five_hours() {
        assert_eq!(local_date("2026-11-10T08:30:00.000-05:00").unwrap(), "2026-11-10");
        // 02:00 UTC is the previous night in Ecuador.
        assert_eq!(local_date("2026-11-10T02:00:00.000Z").unwrap(), "2026-11-09");
        assert_eq!(local_date("2026-11-10T04:59:59.999Z").unwrap(), "2026-11-09");
        assert_eq!(local_date("2026-11-10T05:00:00.000Z").unwrap(), "2026-11-10");
        // Year change.
        assert_eq!(local_date("2027-01-01T03:00:00.000Z").unwrap(), "2026-12-31");
        // A bare date is midnight UTC: in Ecuador it falls on the previous day (same as in JS).
        assert_eq!(local_date("2026-11-10").unwrap(), "2026-11-09");
        assert!(local_date("no es fecha").is_err());
    }

    #[test]
    fn date_only_respects_the_length() {
        assert_eq!(date_only(None).unwrap(), None);
        assert_eq!(date_only(Some("")).unwrap(), None);
        assert_eq!(date_only(Some("2026-11-10")).unwrap().as_deref(), Some("2026-11-10"));
        assert_eq!(date_only(Some("2026-11-10T01:00:00.000+00:00")).unwrap().as_deref(), Some("2026-11-09"));
    }

    #[test]
    fn monday_and_day_addition() {
        assert_eq!(monday_of("2026-11-15").unwrap(), "2026-11-09"); // Sunday
        assert_eq!(monday_of("2026-11-09").unwrap(), "2026-11-09");
        assert_eq!(monday_of("2027-01-01").unwrap(), "2026-12-28");
        assert_eq!(add_days("2026-12-30", 3).unwrap(), "2027-01-02");
        assert_eq!(add_days("2026-03-01", -1).unwrap(), "2026-02-28");
        assert!(add_days("2026-13-01", 1).is_err());
        assert!(is_monday("2026-11-09").unwrap());
        assert!(!is_monday("2026-11-10").unwrap());
    }

    #[test]
    fn iso_week_with_year_change() {
        assert_eq!(iso_week("2026-11-09").unwrap(), (2026, 46));
        assert_eq!(iso_week("2026-12-28").unwrap(), (2026, 53));
        assert_eq!(iso_week("2027-01-01").unwrap(), (2026, 53));
        assert_eq!(iso_week("2027-01-04").unwrap(), (2027, 1));
        assert_eq!(iso_week("2024-12-30").unwrap(), (2025, 1));
        assert_eq!(iso_week("2026-01-01").unwrap(), (2026, 1));
    }

    #[test]
    fn date_and_hour_formatting() {
        assert_eq!(fmt_d(Some("2026-11-03")), "3 nov");
        assert_eq!(fmt_d(Some("2026-12-25")), "25 dic");
        assert_eq!(fmt_d(None), "—");
        assert_eq!(fh(27000), "7.5 h");
        assert_eq!(fh(28800), "8 h");
        assert_eq!(fh(0), "0 h");
        assert_eq!(fh(3600 * 40 + 1800), "40.5 h");
        // 54 s = 0.15 tenths of 360 s → 0; 180 s = 0.5 → 1 (halves round up).
        assert_eq!(tenths(179), 0);
        assert_eq!(tenths(180), 1);
        assert_eq!(fh(180), "0.1 h");
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(2.5), 3.0);
    }

    #[test]
    fn base_holidays_removed_and_added() {
        let mut extra = no_extra();
        assert!(is_holiday("2026-11-02", &extra));
        assert!(!is_holiday("2026-11-04", &extra));
        assert_eq!(week_holidays("2026-11-02", &extra).unwrap(), ["2026-11-02", "2026-11-03"]);
        assert_eq!(week_target("2026-11-02", &extra).unwrap(), 24);
        extra.insert("2026-11-02".into(), false); // removes a base one
        extra.insert("2026-11-04".into(), true); // adds one
        assert!(!is_holiday("2026-11-02", &extra));
        assert!(is_holiday("2026-11-04", &extra));
        assert_eq!(week_target("2026-11-02", &extra).unwrap(), 24);
        assert_eq!(workdays("2026-11-02", &extra).unwrap(), ["2026-11-02", "2026-11-05", "2026-11-06"]);
        assert_eq!(week_target("2026-11-09", &no_extra()).unwrap(), 40);
    }
}
