//! Dónde se guarda el token entre una sesión y otra.
//!
//! - [`AlmacenSistema`]: Administrador de credenciales de Windows (en Linux, el keyring del kernel).
//! - [`AlmacenMemoria`]: solo en memoria; se usa en modo simulado y en las pruebas.
//!
//! El sitio y el correo no son secretos y van en `config.toml` (ver `config.rs`).

use std::collections::HashMap;
use std::sync::Mutex;

use secrecy::{ExposeSecret, SecretString};

use crate::error::Resultado;

pub trait AlmacenSecretos: Send + Sync {
    fn guardar(&self, servicio: &str, cuenta: &str, secreto: &SecretString) -> Resultado<()>;
    fn leer(&self, servicio: &str, cuenta: &str) -> Resultado<Option<SecretString>>;
    fn borrar(&self, servicio: &str, cuenta: &str) -> Resultado<()>;
}

/// Nombre con el que aparece la credencial en Windows: `signal-aduana:empresa.atlassian.net`.
pub fn servicio_para(host: &str) -> String {
    format!("signal-aduana:{host}")
}

pub struct AlmacenSistema;

impl AlmacenSecretos for AlmacenSistema {
    fn guardar(&self, servicio: &str, cuenta: &str, secreto: &SecretString) -> Resultado<()> {
        keyring::Entry::new(servicio, cuenta)?.set_password(secreto.expose_secret())?;
        Ok(())
    }

    fn leer(&self, servicio: &str, cuenta: &str) -> Resultado<Option<SecretString>> {
        match keyring::Entry::new(servicio, cuenta)?.get_password() {
            Ok(s) => Ok(Some(SecretString::from(s))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn borrar(&self, servicio: &str, cuenta: &str) -> Resultado<()> {
        match keyring::Entry::new(servicio, cuenta)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Default)]
pub struct AlmacenMemoria {
    datos: Mutex<HashMap<(String, String), SecretString>>,
}

impl AlmacenSecretos for AlmacenMemoria {
    fn guardar(&self, servicio: &str, cuenta: &str, secreto: &SecretString) -> Resultado<()> {
        self.datos
            .lock()
            .expect("mutex")
            .insert((servicio.into(), cuenta.into()), secreto.clone());
        Ok(())
    }

    fn leer(&self, servicio: &str, cuenta: &str) -> Resultado<Option<SecretString>> {
        Ok(self
            .datos
            .lock()
            .expect("mutex")
            .get(&(servicio.into(), cuenta.into()))
            .cloned())
    }

    fn borrar(&self, servicio: &str, cuenta: &str) -> Resultado<()> {
        self.datos.lock().expect("mutex").remove(&(servicio.into(), cuenta.into()));
        Ok(())
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn memoria_guarda_lee_y_borra() {
        let a = AlmacenMemoria::default();
        let s = servicio_para("x.atlassian.net");
        assert!(a.leer(&s, "a@b.com").unwrap().is_none());
        a.guardar(&s, "a@b.com", &SecretString::from("t1")).unwrap();
        assert_eq!(a.leer(&s, "a@b.com").unwrap().unwrap().expose_secret(), "t1");
        a.borrar(&s, "a@b.com").unwrap();
        assert!(a.leer(&s, "a@b.com").unwrap().is_none());
        // Borrar algo que no existe no es error.
        a.borrar(&s, "a@b.com").unwrap();
    }
}
