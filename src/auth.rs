//! Autenticación con Atlassian.
//!
//! Fase 1: API token clásico (correo + token, encabezado `Authorization: Basic`).
//! El diseño deja espacio para otros métodos (token con alcances, OAuth 2.0): basta con
//! agregar una variante a [`Credenciales`] y resolverla en el `match` de [`Autenticador`].
//! El resto del código solo conoce el `trait`.

use reqwest::RequestBuilder;
use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::error::{ErrorApp, Resultado};

/// Sitio de Jira Cloud validado: siempre `https://<algo>.atlassian.net`, sin ruta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sitio(Url);

impl Sitio {
    /// Acepta `torresytorres.atlassian.net`, `https://torresytorres.atlassian.net/` o
    /// una URL de Jira con ruta (se queda solo con el origen).
    pub fn parse(entrada: &str) -> Resultado<Self> {
        let texto = entrada.trim();
        if texto.is_empty() {
            return Err(ErrorApp::SitioInvalido("está vacío".into()));
        }
        let con_esquema = if texto.contains("://") {
            texto.to_string()
        } else {
            format!("https://{texto}")
        };
        let url = Url::parse(&con_esquema).map_err(|e| ErrorApp::SitioInvalido(e.to_string()))?;
        if url.scheme() != "https" {
            return Err(ErrorApp::SitioInvalido("debe usar https".into()));
        }
        let host = url
            .host_str()
            .ok_or_else(|| ErrorApp::SitioInvalido("falta el dominio".into()))?
            .to_ascii_lowercase();
        let sub = host.strip_suffix(".atlassian.net").unwrap_or("");
        if sub.is_empty() || sub.contains('.') || !url.username().is_empty() || url.port().is_some() {
            return Err(ErrorApp::SitioInvalido(
                "debe ser un sitio de Jira Cloud, por ejemplo empresa.atlassian.net".into(),
            ));
        }
        let origen = Url::parse(&format!("https://{host}/")).expect("origen válido");
        Ok(Sitio(origen))
    }

    pub fn url(&self) -> &Url {
        &self.0
    }

    pub fn host(&self) -> &str {
        self.0.host_str().unwrap_or_default()
    }

    /// `https://empresa.atlassian.net` (sin barra final), para armar enlaces en la interfaz.
    pub fn como_texto(&self) -> String {
        self.0.as_str().trim_end_matches('/').to_string()
    }
}

/// Correo con forma mínima válida (no se verifica contra Atlassian hasta iniciar sesión).
pub fn validar_correo(correo: &str) -> Resultado<String> {
    let c = correo.trim();
    let valido = c.len() <= 254
        && !c.contains(char::is_whitespace)
        && c.split_once('@').is_some_and(|(u, d)| !u.is_empty() && d.contains('.') && !d.starts_with('.') && !d.ends_with('.'));
    if valido {
        Ok(c.to_string())
    } else {
        Err(ErrorApp::CorreoInvalido)
    }
}

/// Formas de autenticarse. Por ahora solo existe el token clásico.
#[derive(Clone)]
pub enum Credenciales {
    TokenClasico {
        sitio: Sitio,
        correo: String,
        token: SecretString,
    },
}

impl Credenciales {
    pub fn token_clasico(sitio: Sitio, correo: &str, token: SecretString) -> Resultado<Self> {
        let correo = validar_correo(correo)?;
        if token.expose_secret().trim().is_empty() {
            return Err(ErrorApp::TokenVacio);
        }
        Ok(Credenciales::TokenClasico { sitio, correo, token })
    }
}

/// `Debug` a mano para que el token nunca aparezca en logs ni en `{:?}`.
impl std::fmt::Debug for Credenciales {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Credenciales::TokenClasico { sitio, correo, .. } => f
                .debug_struct("TokenClasico")
                .field("sitio", &sitio.host())
                .field("correo", correo)
                .field("token", &"[oculto]")
                .finish(),
        }
    }
}

/// Lo único que el cliente HTTP necesita saber de la autenticación.
pub trait Autenticador: Send + Sync {
    /// URL base para las APIs REST (`/rest/api/3/...`, `/wiki/rest/api/...`).
    fn url_base(&self) -> &Url;
    /// Agrega el encabezado de autenticación a una petición.
    fn firmar(&self, req: RequestBuilder) -> RequestBuilder;
}

impl Autenticador for Credenciales {
    fn url_base(&self) -> &Url {
        match self {
            Credenciales::TokenClasico { sitio, .. } => sitio.url(),
        }
    }

    fn firmar(&self, req: RequestBuilder) -> RequestBuilder {
        match self {
            Credenciales::TokenClasico { correo, token, .. } => {
                req.basic_auth(correo, Some(token.expose_secret()))
            }
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn sitio_acepta_formas_comunes() {
        for entrada in [
            "torresytorres.atlassian.net",
            "https://torresytorres.atlassian.net",
            "https://TorresYTorres.atlassian.net/browse/APP-1",
            "  torresytorres.atlassian.net/  ",
        ] {
            let s = Sitio::parse(entrada).unwrap_or_else(|e| panic!("{entrada}: {e}"));
            assert_eq!(s.como_texto(), "https://torresytorres.atlassian.net");
        }
    }

    #[test]
    fn sitio_rechaza_lo_que_no_es_jira_cloud() {
        for entrada in [
            "",
            "http://torresytorres.atlassian.net",
            "atlassian.net",
            "torresytorres.atlassian.net.evil.com",
            "a.b.atlassian.net",
            "https://user@torresytorres.atlassian.net",
            "https://torresytorres.atlassian.net:8443",
            "ejemplo.com",
        ] {
            assert!(Sitio::parse(entrada).is_err(), "debería rechazar {entrada:?}");
        }
    }

    #[test]
    fn correo_valida_forma_minima() {
        assert!(validar_correo(" persona@empresa.com ").is_ok());
        for malo in ["", "persona", "@empresa.com", "persona@empresa", "per sona@empresa.com", "a@.com"] {
            assert!(validar_correo(malo).is_err(), "debería rechazar {malo:?}");
        }
    }

    #[test]
    fn token_vacio_no_crea_credenciales() {
        let sitio = Sitio::parse("x.atlassian.net").unwrap();
        let r = Credenciales::token_clasico(sitio, "a@b.com", SecretString::from("   "));
        assert!(matches!(r, Err(ErrorApp::TokenVacio)));
    }

    #[test]
    fn debug_no_muestra_el_token() {
        let sitio = Sitio::parse("x.atlassian.net").unwrap();
        let c = Credenciales::token_clasico(sitio, "a@b.com", SecretString::from("super-secreto")).unwrap();
        let txt = format!("{c:?}");
        assert!(!txt.contains("super-secreto"));
        assert!(txt.contains("[oculto]"));
    }

    #[test]
    fn firma_con_basic_auth() {
        let sitio = Sitio::parse("x.atlassian.net").unwrap();
        let c = Credenciales::token_clasico(sitio, "a@b.com", SecretString::from("tok")).unwrap();
        let req = c.firmar(reqwest::Client::new().get(c.url_base().clone())).build().unwrap();
        let valor = req.headers().get(reqwest::header::AUTHORIZATION).unwrap().to_str().unwrap();
        // base64("a@b.com:tok") = YUBiLmNvbTp0b2s=
        assert_eq!(valor, "Basic YUBiLmNvbTp0b2s=");
        assert!(req.headers().get(reqwest::header::AUTHORIZATION).unwrap().is_sensitive());
    }
}
