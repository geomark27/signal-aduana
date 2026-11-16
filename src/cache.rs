//! Caché local: la última carga completa del tablero, para mostrarla al abrir y sin red.
//! En Windows queda en `%LOCALAPPDATA%\ec.com.torresytorres.signal\cache.json`.
//!
//! `datos` es opaco para Rust: la interfaz decide qué guarda. El archivo nunca contiene el token
//! ni el correo; solo el `account_id` y el sitio, para no mostrar datos de otra cuenta.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{ErrorApp, Resultado};

/// Versión del formato del archivo. Si cambia, las cachés anteriores se ignoran.
const VERSION: u32 = 1;

/// Tamaño máximo del archivo serializado: 20 MB.
pub const MAX_BYTES: usize = 20 * 1024 * 1024;

/// Lo que se devuelve a la interfaz (Contrato C).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Instantanea {
    /// Fecha-hora RFC 3339 (UTC) en que se escribió el archivo.
    pub guardado: String,
    pub datos: Value,
}

#[derive(Serialize, Deserialize)]
struct Archivo {
    version: u32,
    account_id: String,
    sitio: String,
    guardado: String,
    datos: Value,
}

pub fn ruta(dir: &Path, simulado: bool) -> PathBuf {
    dir.join(if simulado { "cache.simulado.json" } else { "cache.json" })
}

/// Guarda `datos` de forma atómica (temporal + renombrar).
pub fn guardar(dir: &Path, simulado: bool, account_id: &str, sitio: &str, datos: Value) -> Resultado<()> {
    let archivo = Archivo {
        version: VERSION,
        account_id: account_id.to_string(),
        sitio: sitio.to_string(),
        guardado: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        datos,
    };
    let texto = serde_json::to_vec(&archivo).map_err(|e| ErrorApp::Cache(e.to_string()))?;
    if texto.len() > MAX_BYTES {
        return Err(ErrorApp::Cache(format!(
            "Los datos guardados superan el tamaño máximo de {} MB.",
            MAX_BYTES / (1024 * 1024)
        )));
    }
    fs::create_dir_all(dir).map_err(|e| ErrorApp::Cache(e.to_string()))?;
    let destino = ruta(dir, simulado);
    let tmp = destino.with_extension("json.tmp");
    fs::write(&tmp, texto).map_err(|e| ErrorApp::Cache(e.to_string()))?;
    fs::rename(&tmp, &destino).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        ErrorApp::Cache(e.to_string())
    })
}

/// `None` si no hay archivo, está dañado, es de otra versión o es de otra cuenta o sitio.
pub fn leer(dir: &Path, simulado: bool, account_id: &str, sitio: &str) -> Option<Instantanea> {
    let bytes = fs::read(ruta(dir, simulado)).ok()?;
    let a: Archivo = serde_json::from_slice(&bytes).ok()?;
    if a.version != VERSION || a.account_id != account_id || a.sitio != sitio {
        return None;
    }
    Some(Instantanea { guardado: a.guardado, datos: a.datos })
}

/// Borra la caché; que no exista no es error.
pub fn borrar(dir: &Path, simulado: bool) {
    let _ = fs::remove_file(ruta(dir, simulado));
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use serde_json::json;

    const SITIO: &str = "https://x.atlassian.net";

    #[test]
    fn ida_y_vuelta() {
        let dir = tempfile::tempdir().unwrap();
        assert!(leer(dir.path(), false, "u1", SITIO).is_none());
        let datos = json!({ "formato": 1, "issues": [{ "key": "APP-1" }] });
        guardar(dir.path(), false, "u1", SITIO, datos.clone()).unwrap();
        let i = leer(dir.path(), false, "u1", SITIO).unwrap();
        assert_eq!(i.datos, datos);
        assert!(chrono::DateTime::parse_from_rfc3339(&i.guardado).is_ok());
        assert!(i.guardado.ends_with('Z'));
        let texto = fs::read_to_string(ruta(dir.path(), false)).unwrap();
        assert!(!texto.contains("token") && !texto.contains('@'));
    }

    #[test]
    fn simulado_usa_otro_archivo() {
        let dir = tempfile::tempdir().unwrap();
        guardar(dir.path(), true, "u1", SITIO, json!(1)).unwrap();
        assert!(ruta(dir.path(), true).ends_with("cache.simulado.json"));
        assert!(leer(dir.path(), true, "u1", SITIO).is_some());
        assert!(leer(dir.path(), false, "u1", SITIO).is_none());
    }

    #[test]
    fn otra_cuenta_o_sitio_da_none() {
        let dir = tempfile::tempdir().unwrap();
        guardar(dir.path(), false, "u1", SITIO, json!({ "a": 1 })).unwrap();
        assert!(leer(dir.path(), false, "u2", SITIO).is_none());
        assert!(leer(dir.path(), false, "u1", "https://y.atlassian.net").is_none());
        assert!(leer(dir.path(), false, "u1", SITIO).is_some());
    }

    #[test]
    fn archivo_danado_da_none() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(ruta(dir.path(), false), "esto no es json {{{").unwrap();
        assert!(leer(dir.path(), false, "u1", SITIO).is_none());
        fs::write(ruta(dir.path(), false), r#"{"version":1}"#).unwrap();
        assert!(leer(dir.path(), false, "u1", SITIO).is_none());
    }

    #[test]
    fn version_distinta_da_none() {
        let dir = tempfile::tempdir().unwrap();
        let otro = json!({ "version": 2, "account_id": "u1", "sitio": SITIO,
            "guardado": "2026-01-01T00:00:00Z", "datos": {} });
        fs::write(ruta(dir.path(), false), otro.to_string()).unwrap();
        assert!(leer(dir.path(), false, "u1", SITIO).is_none());
    }

    #[test]
    fn rechaza_datos_demasiado_grandes() {
        let dir = tempfile::tempdir().unwrap();
        let grande = Value::String("a".repeat(MAX_BYTES + 1));
        let r = guardar(dir.path(), false, "u1", SITIO, grande);
        assert!(matches!(r, Err(ErrorApp::Cache(_))));
        assert!(!ruta(dir.path(), false).exists());
    }

    #[test]
    fn borrar_no_falla_si_no_existe() {
        let dir = tempfile::tempdir().unwrap();
        borrar(dir.path(), false);
        guardar(dir.path(), false, "u1", SITIO, json!(1)).unwrap();
        borrar(dir.path(), false);
        assert!(leer(dir.path(), false, "u1", SITIO).is_none());
    }

    #[test]
    fn crea_el_directorio_y_no_deja_temporales() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("nuevo").join("anidado");
        guardar(&sub, false, "u1", SITIO, json!(1)).unwrap();
        let nombres: Vec<_> = fs::read_dir(&sub).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(nombres, vec![std::ffi::OsString::from("cache.json")]);
    }
}
