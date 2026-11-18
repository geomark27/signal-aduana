//! Application errors.
//!
//! Each variant has a stable `code` that the UI uses to decide what to do (for example,
//! `unauthorized` returns to the connection screen) and a Spanish message ready to display.
//! No message includes the token or authentication headers.

use serde::ser::{Serialize, SerializeStruct, Serializer};

pub type AppResult<T> = std::result::Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("El sitio no es válido: {0}")]
    InvalidSite(String),

    #[error("El correo no es válido.")]
    InvalidEmail,

    #[error("Falta el API token.")]
    EmptyToken,

    #[error("No hay una sesión iniciada.")]
    NoSession,

    #[error("Jira rechazó las credenciales: el token está vencido, revocado o es incorrecto.")]
    Unauthorized,

    #[error("Tu usuario no tiene permiso para esta consulta en Jira.")]
    Forbidden,

    #[error("Jira limitó la cantidad de consultas. Intenta de nuevo en unos segundos.")]
    RateLimited,

    #[error("No se pudo conectar con Atlassian: {0}")]
    Network(String),

    #[error("Jira respondió {status}: {detail}")]
    Jira { status: u16, detail: String },

    #[error("Respuesta de Jira no reconocida: {0}")]
    Format(String),

    #[error("Clave de tarea no válida: {0}")]
    InvalidKey(String),

    #[error("No se pudo usar el almacén de credenciales del sistema: {0}")]
    SecretStore(String),

    #[error("No se pudo leer o guardar la configuración: {0}")]
    Config(String),

    #[error("No se pudo guardar la caché local: {0}")]
    Cache(String),

    #[error("No se pudo actualizar: {0}")]
    Update(String),
}

impl AppError {
    /// Stable code for the UI.
    pub fn code(&self) -> &'static str {
        match self {
            AppError::InvalidSite(_) => "invalid_site",
            AppError::InvalidEmail => "invalid_email",
            AppError::EmptyToken => "empty_token",
            AppError::NoSession => "no_session",
            AppError::Unauthorized => "unauthorized",
            AppError::Forbidden => "forbidden",
            AppError::RateLimited => "rate_limit",
            AppError::Network(_) => "network",
            AppError::Jira { .. } => "jira",
            AppError::Format(_) => "format",
            AppError::InvalidKey(_) => "invalid_key",
            AppError::SecretStore(_) => "secret_store",
            AppError::Config(_) => "config",
            AppError::Cache(_) => "cache",
            AppError::Update(_) => "update",
        }
    }
}

/// Tauri sends command errors to JS serialized: `{ code, message }`.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("AppError", 2)?;
        st.serialize_field("code", self.code())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        // `without_url` keeps query parameters out of the message.
        let e = e.without_url();
        if e.is_timeout() {
            AppError::Network("Atlassian no respondió a tiempo.".into())
        } else if e.is_decode() {
            AppError::Format(e.to_string())
        } else {
            AppError::Network(e.to_string())
        }
    }
}

impl From<keyring::Error> for AppError {
    fn from(e: keyring::Error) -> Self {
        AppError::SecretStore(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_code_and_message() {
        let json = serde_json::to_value(AppError::Unauthorized).unwrap();
        assert_eq!(json["code"], "unauthorized");
        assert!(json["message"].as_str().unwrap().contains("token"));
    }
}
