//! Preferencias del usuario, guardadas en el equipo (no en el navegador).
//! En Windows quedan en `%APPDATA%\ec.com.torresytorres.signal\preferencias.toml`.
//! No dependen de la cuenta: no se borran al cerrar sesión.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::dominio::calendario::{validar_fecha, FeriadosExtra};
use crate::error::{ErrorApp, Resultado};

pub const HORA_CIERRE_DEFECTO: &str = "16:30";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PrefAvisos {
    pub activos: bool,
    /// `HH:MM`, hora de Ecuador.
    pub hora_cierre: String,
    pub resumen_lunes: bool,
}

impl Default for PrefAvisos {
    fn default() -> Self {
        PrefAvisos { activos: true, hora_cierre: HORA_CIERRE_DEFECTO.into(), resumen_lunes: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Preferencias {
    pub avisos: PrefAvisos,
    pub iniciar_con_windows: bool,
    pub minimizar_a_bandeja: bool,
    pub feriados_extra: FeriadosExtra,
}

impl Default for Preferencias {
    fn default() -> Self {
        Preferencias {
            avisos: PrefAvisos::default(),
            iniciar_con_windows: false,
            minimizar_a_bandeja: true,
            feriados_extra: FeriadosExtra::new(),
        }
    }
}

pub fn ruta(dir: &Path, simulado: bool) -> PathBuf {
    dir.join(if simulado { "preferencias.simulado.toml" } else { "preferencias.toml" })
}

/// Valida `HH:MM` (00:00 a 23:59, dos dígitos cada parte) y lo devuelve igual.
fn validar_hora(hora: &str) -> Resultado<String> {
    // Sin índices ni cortes de texto: se separa en `HH` y `MM` y cada parte debe ser de dos dígitos.
    let dos_digitos = |t: &str| t.chars().count() == 2 && t.chars().all(|c| c.is_ascii_digit());
    let valida = hora.split_once(':').is_some_and(|(h, m)| {
        dos_digitos(h)
            && dos_digitos(m)
            && h.parse::<u32>().is_ok_and(|h| h < 24)
            && m.parse::<u32>().is_ok_and(|m| m < 60)
    });
    if valida {
        Ok(hora.to_string())
    } else {
        Err(ErrorApp::Formato(format!(
            "La hora de cierre debe ser HH:MM entre 00:00 y 23:59: «{}»",
            hora.chars().take(20).collect::<String>()
        )))
    }
}

impl Preferencias {
    /// Hora inválida → `Formato`; los feriados con fecha inválida se descartan.
    pub fn normalizar(mut self) -> Resultado<Preferencias> {
        self.avisos.hora_cierre = validar_hora(self.avisos.hora_cierre.trim())?;
        self.feriados_extra = self
            .feriados_extra
            .into_iter()
            .filter_map(|(k, v)| validar_fecha(&k).ok().map(|f| (f, v)))
            .collect();
        Ok(self)
    }
}

/// Sin archivo o dañado → valores por defecto. Si solo la hora guardada es inválida, se repone esa.
pub fn cargar(dir: &Path, simulado: bool) -> Preferencias {
    let leidas: Option<Preferencias> =
        fs::read_to_string(ruta(dir, simulado)).ok().and_then(|t| toml::from_str(&t).ok());
    let Some(mut p) = leidas else { return Preferencias::default() };
    if validar_hora(p.avisos.hora_cierre.trim()).is_err() {
        p.avisos.hora_cierre = HORA_CIERRE_DEFECTO.into();
    }
    p.normalizar().unwrap_or_default()
}

/// Normaliza y guarda (escritura atómica). Devuelve lo que quedó guardado.
pub fn guardar(dir: &Path, simulado: bool, p: &Preferencias) -> Resultado<Preferencias> {
    let p = p.clone().normalizar()?;
    fs::create_dir_all(dir).map_err(|e| ErrorApp::Config(e.to_string()))?;
    let texto = toml::to_string_pretty(&p).map_err(|e| ErrorApp::Config(e.to_string()))?;
    let destino = ruta(dir, simulado);
    let tmp = destino.with_extension("toml.tmp");
    fs::write(&tmp, texto).map_err(|e| ErrorApp::Config(e.to_string()))?;
    fs::rename(&tmp, &destino).map_err(|e| ErrorApp::Config(e.to_string()))?;
    Ok(p)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn defectos() {
        let dir = tempfile::tempdir().unwrap();
        let p = cargar(dir.path(), false);
        assert!(p.avisos.activos && p.avisos.resumen_lunes && p.minimizar_a_bandeja);
        assert_eq!(p.avisos.hora_cierre, "16:30");
        assert!(!p.iniciar_con_windows);
        assert!(p.feriados_extra.is_empty());
        let j = serde_json::to_value(&p).unwrap();
        assert_eq!(j["avisos"]["horaCierre"], "16:30");
        assert_eq!(j["iniciarConWindows"], false);
        assert_eq!(j["minimizarABandeja"], true);
        assert!(j["feriadosExtra"].is_object());
    }

    #[test]
    fn ida_y_vuelta() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = Preferencias::default();
        p.avisos.hora_cierre = "17:05".into();
        p.avisos.activos = false;
        p.iniciar_con_windows = true;
        p.feriados_extra.insert("2026-12-24".into(), true);
        p.feriados_extra.insert("2026-11-03".into(), false);
        let guardada = guardar(dir.path(), false, &p).unwrap();
        assert_eq!(guardada, p);
        assert_eq!(cargar(dir.path(), false), p);
        // El simulado usa otro archivo.
        assert_eq!(cargar(dir.path(), true), Preferencias::default());
        assert!(ruta(dir.path(), true).to_string_lossy().ends_with("preferencias.simulado.toml"));
    }

    #[test]
    fn archivo_danado_da_defectos() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(ruta(dir.path(), false), "esto no es toml [[[").unwrap();
        assert_eq!(cargar(dir.path(), false), Preferencias::default());
    }

    #[test]
    fn archivo_parcial_completa_con_defectos() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(ruta(dir.path(), false), "iniciarConWindows = true\n").unwrap();
        let p = cargar(dir.path(), false);
        assert!(p.iniciar_con_windows);
        assert_eq!(p.avisos, PrefAvisos::default());
    }

    #[test]
    fn hora_invalida_guardada_se_repone() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(ruta(dir.path(), false), "iniciarConWindows = true\n[avisos]\nhoraCierre = \"25:99\"\n").unwrap();
        let p = cargar(dir.path(), false);
        assert_eq!(p.avisos.hora_cierre, "16:30");
        assert!(p.iniciar_con_windows);
    }

    #[test]
    fn hora_invalida_al_guardar_es_error_de_formato() {
        let dir = tempfile::tempdir().unwrap();
        for mala in [
            "24:00", "12:60", "9:30", "1230", "ab:cd", "", "12:3", "12:30:00", "+1:30", ":", "12:", ":30",
            "１２:３０", "é1:30", "12:3é", "12 :30",
        ] {
            let mut p = Preferencias::default();
            p.avisos.hora_cierre = mala.into();
            let e = guardar(dir.path(), false, &p).unwrap_err();
            assert_eq!(e.codigo(), "formato", "{mala:?}");
        }
        assert!(!ruta(dir.path(), false).exists());
        // Los espacios de los extremos se recortan antes de validar.
        for buena in ["00:00", "23:59", "08:05", " 12:30 "] {
            let mut p = Preferencias::default();
            p.avisos.hora_cierre = buena.into();
            assert!(p.normalizar().is_ok(), "{buena}");
        }
    }

    #[test]
    fn feriado_invalido_se_descarta() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = Preferencias::default();
        p.feriados_extra.insert("2026-02-30".into(), true);
        p.feriados_extra.insert("mañana".into(), true);
        p.feriados_extra.insert("2026-1-5".into(), true);
        p.feriados_extra.insert("2026-12-24".into(), true);
        let g = guardar(dir.path(), false, &p).unwrap();
        assert_eq!(g.feriados_extra.len(), 1);
        assert_eq!(g.feriados_extra.get("2026-12-24"), Some(&true));
        assert_eq!(cargar(dir.path(), false), g);
    }

    #[test]
    fn acepta_el_json_de_la_interfaz() {
        let j = serde_json::json!({
            "avisos": { "activos": true, "horaCierre": "16:30", "resumenLunes": false },
            "iniciarConWindows": false, "minimizarABandeja": true,
            "feriadosExtra": { "2026-12-24": true }
        });
        let p: Preferencias = serde_json::from_value(j).unwrap();
        assert!(!p.avisos.resumen_lunes);
        assert!(p.feriados_extra["2026-12-24"]);
    }
}
