//! Vuelca en JSON las tareas del Jira simulado para un `hoy` dado, tal como las tiene la interfaz
//! en memoria tras `jira::cargar_para_resumen` (equivale a `cargar_tareas` + `claves_con_horas` + `leer_horas`) más `hasPR`.
//!
//! Uso: `cargo run --example volcar_simulado -- 2026-11-13 > volcado.json`
//! Solo usa el simulado: no hay ninguna conexión de red.

use chrono::NaiveDate;
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

    // Tareas con horas unidas, como las tiene la interfaz (la misma función que usa el monitor).
    let mut tareas = jira::cargar_para_resumen(&fuente, Some(&yo), hoy).await?;

    // El volcado de paridad además incluye `hasPR`.
    let con_pr = jira::claves_con_pr(&fuente).await?;
    for t in &mut tareas {
        let clave = t.get("key").and_then(Value::as_str).unwrap_or_default().to_string();
        t.insert("hasPR".into(), json!(con_pr.contains(&clave)));
    }
    Ok(Value::Array(tareas.into_iter().map(Value::Object).collect()))
}
