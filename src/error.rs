//! Errores de la aplicación.
//!
//! Cada variante tiene un `codigo` estable que la interfaz usa para decidir qué hacer
//! (por ejemplo, `no_autorizado` vuelve a la pantalla de conexión) y un mensaje en español
//! listo para mostrar. Ningún mensaje incluye el token ni encabezados de autenticación.

use serde::ser::{Serialize, SerializeStruct, Serializer};

pub type Resultado<T> = std::result::Result<T, ErrorApp>;

#[derive(Debug, thiserror::Error)]
pub enum ErrorApp {
    #[error("El sitio no es válido: {0}")]
    SitioInvalido(String),

    #[error("El correo no es válido.")]
    CorreoInvalido,

    #[error("Falta el API token.")]
    TokenVacio,

    #[error("No hay una sesión iniciada.")]
    SinSesion,

    #[error("Jira rechazó las credenciales: el token está vencido, revocado o es incorrecto.")]
    NoAutorizado,

    #[error("Tu usuario no tiene permiso para esta consulta en Jira.")]
    SinPermiso,

    #[error("Jira limitó la cantidad de consultas. Intenta de nuevo en unos segundos.")]
    Limite,

    #[error("No se pudo conectar con Atlassian: {0}")]
    Red(String),

    #[error("Jira respondió {estado}: {detalle}")]
    Jira { estado: u16, detalle: String },

    #[error("Respuesta de Jira no reconocida: {0}")]
    Formato(String),

    #[error("Clave de tarea no válida: {0}")]
    ClaveInvalida(String),

    #[error("No se pudo usar el almacén de credenciales del sistema: {0}")]
    Almacen(String),

    #[error("No se pudo leer o guardar la configuración: {0}")]
    Config(String),

    #[error("No se pudo guardar la caché local: {0}")]
    Cache(String),
}

impl ErrorApp {
    /// Código estable para la interfaz.
    pub fn codigo(&self) -> &'static str {
        match self {
            ErrorApp::SitioInvalido(_) => "sitio_invalido",
            ErrorApp::CorreoInvalido => "correo_invalido",
            ErrorApp::TokenVacio => "token_vacio",
            ErrorApp::SinSesion => "sin_sesion",
            ErrorApp::NoAutorizado => "no_autorizado",
            ErrorApp::SinPermiso => "sin_permiso",
            ErrorApp::Limite => "limite",
            ErrorApp::Red(_) => "red",
            ErrorApp::Jira { .. } => "jira",
            ErrorApp::Formato(_) => "formato",
            ErrorApp::ClaveInvalida(_) => "clave_invalida",
            ErrorApp::Almacen(_) => "almacen",
            ErrorApp::Config(_) => "config",
            ErrorApp::Cache(_) => "cache",
        }
    }
}

/// Tauri envía los errores de los comandos a JS serializados: `{ codigo, mensaje }`.
impl Serialize for ErrorApp {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("ErrorApp", 2)?;
        st.serialize_field("codigo", self.codigo())?;
        st.serialize_field("mensaje", &self.to_string())?;
        st.end()
    }
}

impl From<reqwest::Error> for ErrorApp {
    fn from(e: reqwest::Error) -> Self {
        // `without_url` evita mostrar parámetros de la consulta en el mensaje.
        let e = e.without_url();
        if e.is_timeout() {
            ErrorApp::Red("Atlassian no respondió a tiempo.".into())
        } else if e.is_decode() {
            ErrorApp::Formato(e.to_string())
        } else {
            ErrorApp::Red(e.to_string())
        }
    }
}

impl From<keyring::Error> for ErrorApp {
    fn from(e: keyring::Error) -> Self {
        ErrorApp::Almacen(e.to_string())
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn serializa_codigo_y_mensaje() {
        let json = serde_json::to_value(ErrorApp::NoAutorizado).unwrap();
        assert_eq!(json["codigo"], "no_autorizado");
        assert!(json["mensaje"].as_str().unwrap().contains("token"));
    }
}
