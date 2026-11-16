//! Configuración no secreta: sitio y correo de la última sesión.
//! En Windows queda en `%APPDATA%\ec.com.torresytorres.signal\config.toml`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{ErrorApp, Resultado};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub sitio: Option<String>,
    pub correo: Option<String>,
}

pub fn ruta(dir: &Path, simulado: bool) -> PathBuf {
    dir.join(if simulado { "config.simulado.toml" } else { "config.toml" })
}

impl Config {
    /// Si el archivo no existe o está dañado, se empieza con una configuración vacía.
    pub fn cargar(ruta: &Path) -> Config {
        fs::read_to_string(ruta)
            .ok()
            .and_then(|t| toml::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn guardar(&self, ruta: &Path) -> Resultado<()> {
        if let Some(dir) = ruta.parent() {
            fs::create_dir_all(dir).map_err(|e| ErrorApp::Config(e.to_string()))?;
        }
        let texto = toml::to_string_pretty(self).map_err(|e| ErrorApp::Config(e.to_string()))?;
        // Se escribe en un temporal y luego se renombra, para no dejar el archivo a medias.
        let tmp = ruta.with_extension("toml.tmp");
        fs::write(&tmp, texto).map_err(|e| ErrorApp::Config(e.to_string()))?;
        fs::rename(&tmp, ruta).map_err(|e| ErrorApp::Config(e.to_string()))
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn guarda_y_carga() {
        let dir = tempfile::tempdir().unwrap();
        let r = ruta(dir.path(), false);
        assert_eq!(Config::cargar(&r), Config::default());
        let c = Config {
            sitio: Some("https://x.atlassian.net".into()),
            correo: Some("a@b.com".into()),
        };
        c.guardar(&r).unwrap();
        assert_eq!(Config::cargar(&r), c);
        let texto = fs::read_to_string(&r).unwrap();
        assert!(!texto.contains("token"));
    }

    #[test]
    fn archivo_danado_no_rompe() {
        let dir = tempfile::tempdir().unwrap();
        let r = ruta(dir.path(), true);
        fs::write(&r, "esto no es toml [[[").unwrap();
        assert_eq!(Config::cargar(&r), Config::default());
    }
}
