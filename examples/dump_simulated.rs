//! Dumps as JSON the issues of the simulated Jira for a given `today`, as the UI holds them
//! in memory after `jira::load_for_summary` (equivalent to `load_issues` + `keys_with_hours` + `read_worklogs`) plus `hasPR`.
//!
//! Usage: `cargo run --example dump_simulated -- 2026-11-13 > tests/parity/dumps/friday.json`
//! (and `2026-11-16` → `tests/parity/dumps/monday.json`)
//! It only uses the simulated source: there is no network connection.

use chrono::NaiveDate;
use serde_json::{json, Value};
use signal_aduana::jira::simulated::SimulatedJira;
use signal_aduana::jira::{self, Source};

#[tokio::main]
async fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("uso: dump_simulated AAAA-MM-DD");
        std::process::exit(2);
    });
    let today = NaiveDate::parse_from_str(&arg, "%Y-%m-%d").unwrap_or_else(|e| {
        eprintln!("fecha no válida «{arg}»: {e}");
        std::process::exit(2);
    });
    match dump(today).await {
        Ok(v) => println!("{}", serde_json::to_string_pretty(&v).expect("JSON serializable")),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

async fn dump(today: NaiveDate) -> Result<Value, Box<dyn std::error::Error>> {
    let source = Source::Simulated(SimulatedJira::new("persona@ejemplo.com", true, today));
    let me = source.myself().await?.account_id;

    // Issues with joined worklogs, as the UI holds them (the same function the monitor uses).
    let mut issues = jira::load_for_summary(&source, Some(&me), today).await?;

    // The parity dump also includes `hasPR`.
    let with_pr = jira::keys_with_pr(&source).await?;
    for t in &mut issues {
        let key = t.get("key").and_then(Value::as_str).unwrap_or_default().to_string();
        t.insert("hasPR".into(), json!(with_pr.contains(&key)));
    }
    Ok(Value::Array(issues.into_iter().map(Value::Object).collect()))
}
