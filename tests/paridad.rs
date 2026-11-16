//! Paridad: la vista calculada en Rust debe ser idéntica (como JSON) a la que produce el JS
//! legado (`pruebas/paridad/legado.js`) para los mismos datos. Los casos los genera
//! `node pruebas/paridad/generar.mjs`.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use signal_aduana::dominio::{calcular_vista, Entrada};

/// Primera ruta JSON en la que difieren dos valores.
fn primera_diferencia(ruta: &str, rust: &Value, js: &Value) -> Option<String> {
    match (rust, js) {
        (Value::Object(a), Value::Object(b)) => {
            let mut claves: Vec<&String> = a.keys().chain(b.keys()).collect();
            claves.sort();
            claves.dedup();
            for k in claves {
                let r = format!("{ruta}/{k}");
                match (a.get(k), b.get(k)) {
                    (Some(x), Some(y)) => {
                        if let Some(d) = primera_diferencia(&r, x, y) {
                            return Some(d);
                        }
                    }
                    (Some(x), None) => return Some(format!("{r}: Rust trae {x}, el JS no tiene el campo")),
                    (None, Some(y)) => return Some(format!("{r}: el JS trae {y}, Rust no tiene el campo")),
                    (None, None) => {}
                }
            }
            None
        }
        (Value::Array(a), Value::Array(b)) => {
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                if let Some(d) = primera_diferencia(&format!("{ruta}/{i}"), x, y) {
                    return Some(d);
                }
            }
            (a.len() != b.len()).then(|| format!("{ruta}: largo distinto (Rust {}, JS {})", a.len(), b.len()))
        }
        _ => (rust != js).then(|| format!("{ruta}: Rust {rust} != JS {js}")),
    }
}

#[test]
fn rust_da_lo_mismo_que_el_js_legado() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("pruebas/paridad/casos");
    let mut entradas: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("no se pudo leer {}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with(".entrada.json")))
        .collect();
    entradas.sort();
    assert!(!entradas.is_empty(), "no hay casos de paridad en {}", dir.display());

    let mut fallos = Vec::new();
    for ruta in &entradas {
        let nombre = ruta.file_name().unwrap().to_str().unwrap().trim_end_matches(".entrada.json").to_string();
        let esperado_ruta = ruta.with_file_name(format!("{nombre}.esperado.json"));
        let entrada: Entrada = serde_json::from_str(&fs::read_to_string(ruta).unwrap())
            .unwrap_or_else(|e| panic!("caso {nombre}: entrada no válida: {e}"));
        let esperado: Value = serde_json::from_str(
            &fs::read_to_string(&esperado_ruta).unwrap_or_else(|e| panic!("caso {nombre}: falta {}: {e}", esperado_ruta.display())),
        )
        .unwrap();
        let hoy = entrada.hoy.clone().unwrap_or_else(|| "2026-11-13".into());
        let vista = calcular_vista(entrada, &hoy).unwrap_or_else(|e| panic!("caso {nombre}: error al calcular: {e}"));
        let obtenido = serde_json::to_value(&vista).unwrap();
        if obtenido != esperado {
            let d = primera_diferencia("", &obtenido, &esperado).unwrap_or_else(|| "(diferencia no localizada)".into());
            fallos.push(format!("caso {nombre}: {d}"));
        }
    }
    assert!(fallos.is_empty(), "{} de {} casos difieren:\n{}", fallos.len(), entradas.len(), fallos.join("\n"));
    println!("{} casos de paridad en verde", entradas.len());
}
