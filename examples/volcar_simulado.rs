//! Vuelca en JSON las tareas del Jira simulado para un `hoy` dado, tal como las tiene la interfaz
//! en memoria tras `cargar_tareas` + `claves_con_horas` + `leer_horas` (ver `fetchWorklogs` en `ui/app.js`).
//!
//! Uso: `cargo run --example volcar_simulado -- 2026-11-13 > volcado.json`
//! Solo usa el simulado: no hay ninguna conexión de red.

use chrono::{Datelike, Duration, NaiveDate};
use serde_json::{json, Value};
use signal_aduana::jira::simulado::JiraSimulado;
use signal_aduana::jira::{self, Fuente};

#[tokio::main]
async fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("uso: volcar_simulado AAAA-MM-DD");
        std::process::exit(2);
    });
    let hoy = NaiveDate::parse_from_str(&arg, "%Y-%m-%d").unwrap_or_else(|e| {
        eprintln!("fecha no válida «{arg}»: {e}");
        std::process::exit(2);
    });
    match volcar(hoy).await {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).expect("JSON serializable")),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

async fn volcar(hoy: NaiveDate) -> Result<Value, Box<dyn std::error::Error>> {
    let fuente = Fuente::Simulado(JiraSimulado::nuevo("persona@ejemplo.com", true, hoy));
    let yo = fuente.yo().await?.account_id;

    let mut tareas: Vec<Value> = jira::cargar_tareas(&fuente, Some(&yo))
        .await?
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<_, _>>()?;

    // Igual que `fetchWorklogs`: lunes de (hoy - 98 días); `after` = ese día a las 00:00 en UTC-5.
    let atras = hoy - Duration::days(98);
    let desde_lunes = atras - Duration::days(i64::from(atras.weekday().num_days_from_monday()));
    let desde = desde_lunes.format("%Y-%m-%d").to_string();
    let despues_ms = desde_lunes.and_hms_opt(5, 0, 0).expect("hora válida").and_utc().timestamp_millis();

    let claves = jira::claves_con_horas(&fuente, &desde).await?;
    let horas = if claves.is_empty() {
        None
    } else {
        Some(jira::leer_horas(&fuente, &claves, despues_ms, Some(&yo)).await?)
    };
    let con_pr = jira::claves_con_pr(&fuente).await?;

    for t in &mut tareas {
        let clave = t["key"].as_str().unwrap_or_default().to_string();
        let wl = horas
            .as_ref()
            .and_then(|h| h.por_clave.get(&clave))
            .map(serde_json::to_value)
            .transpose()?
            .unwrap_or_else(|| json!([]));
        let suma: i64 = wl.as_array().map(|a| a.iter().filter_map(|w| w["s"].as_i64()).sum()).unwrap_or(0);
        t["wlTotal"] = json!(wl.as_array().map_or(0, Vec::len));
        if t["spent"].as_i64().unwrap_or(0) == 0 {
            t["spent"] = json!(suma);
        }
        t["worklogs"] = wl;
        t["hasPR"] = json!(con_pr.contains(&clave));
    }
    Ok(Value::Array(tareas))
}
