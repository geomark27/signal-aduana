//! Authentication with Atlassian.
//!
//! Phase 1: classic API token (email + token, `Authorization: Basic` header).
//! The design leaves room for other methods (scoped token, OAuth 2.0): add a variant to
//! [`Credentials`] and resolve it in the `match` of [`Authenticator`].
//! The rest of the code only knows the `trait`.

use reqwest::RequestBuilder;
use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::error::{AppError, AppResult};

/// Validated Jira Cloud site: always `https://<something>.atlassian.net`, without a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site(Url);

impl Site {
    /// Accepts `torresytorres.atlassian.net`, `https://torresytorres.atlassian.net/` or
    /// a Jira URL with a path (only the origin is kept).
    pub fn parse(input: &str) -> AppResult<Self> {
        let text = input.trim();
        if text.is_empty() {
            return Err(AppError::InvalidSite("está vacío".into()));
        }
        let with_scheme = if text.contains("://") {
            text.to_string()
        } else {
            format!("https://{text}")
        };
        let url = Url::parse(&with_scheme).map_err(|e| AppError::InvalidSite(e.to_string()))?;
        if url.scheme() != "https" {
            return Err(AppError::InvalidSite("debe usar https".into()));
        }
        let host = url
            .host_str()
            .ok_or_else(|| AppError::InvalidSite("falta el dominio".into()))?
            .to_ascii_lowercase();
        let sub = host.strip_suffix(".atlassian.net").unwrap_or("");
        if sub.is_empty() || sub.contains('.') || !url.username().is_empty() || url.port().is_some() {
            return Err(AppError::InvalidSite(
                "debe ser un sitio de Jira Cloud, por ejemplo empresa.atlassian.net".into(),
            ));
        }
        let origin = Url::parse(&format!("https://{host}/")).expect("valid origin");
        Ok(Site(origin))
    }

    pub fn url(&self) -> &Url {
        &self.0
    }

    pub fn host(&self) -> &str {
        self.0.host_str().unwrap_or_default()
    }

    /// `https://empresa.atlassian.net` (no trailing slash), to build links in the UI.
    pub fn as_text(&self) -> String {
        self.0.as_str().trim_end_matches('/').to_string()
    }
}

/// Email with a minimal valid shape (it is not checked against Atlassian until sign-in).
pub fn validate_email(email: &str) -> AppResult<String> {
    let e = email.trim();
    let valid = e.len() <= 254
        && !e.contains(char::is_whitespace)
        && e.split_once('@').is_some_and(|(u, d)| !u.is_empty() && d.contains('.') && !d.starts_with('.') && !d.ends_with('.'));
    if valid {
        Ok(e.to_string())
    } else {
        Err(AppError::InvalidEmail)
    }
}

/// Ways to authenticate. For now only the classic token exists.
#[derive(Clone)]
pub enum Credentials {
    ClassicToken {
        site: Site,
        email: String,
        token: SecretString,
    },
}

impl Credentials {
    pub fn classic_token(site: Site, email: &str, token: SecretString) -> AppResult<Self> {
        let email = validate_email(email)?;
        if token.expose_secret().trim().is_empty() {
            return Err(AppError::EmptyToken);
        }
        Ok(Credentials::ClassicToken { site, email, token })
    }
}

/// Hand-written `Debug` so the token never appears in logs or in `{:?}`.
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Credentials::ClassicToken { site, email, .. } => f
                .debug_struct("ClassicToken")
                .field("site", &site.host())
                .field("email", email)
                .field("token", &"[hidden]")
                .finish(),
        }
    }
}

/// The only thing the HTTP client needs to know about authentication.
pub trait Authenticator: Send + Sync {
    /// Base URL for the REST APIs (`/rest/api/3/...`, `/wiki/rest/api/...`).
    fn base_url(&self) -> &Url;
    /// Adds the authentication header to a request.
    fn sign(&self, req: RequestBuilder) -> RequestBuilder;
}

impl Authenticator for Credentials {
    fn base_url(&self) -> &Url {
        match self {
            Credentials::ClassicToken { site, .. } => site.url(),
        }
    }

    fn sign(&self, req: RequestBuilder) -> RequestBuilder {
        match self {
            Credentials::ClassicToken { email, token, .. } => {
                req.basic_auth(email, Some(token.expose_secret()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_accepts_common_forms() {
        for input in [
            "torresytorres.atlassian.net",
            "https://torresytorres.atlassian.net",
            "https://TorresYTorres.atlassian.net/browse/APP-1",
            "  torresytorres.atlassian.net/  ",
        ] {
            let s = Site::parse(input).unwrap_or_else(|e| panic!("{input}: {e}"));
            assert_eq!(s.as_text(), "https://torresytorres.atlassian.net");
        }
    }

    #[test]
    fn site_rejects_what_is_not_jira_cloud() {
        for input in [
            "",
            "http://torresytorres.atlassian.net",
            "atlassian.net",
            "torresytorres.atlassian.net.evil.com",
            "a.b.atlassian.net",
            "https://user@torresytorres.atlassian.net",
            "https://torresytorres.atlassian.net:8443",
            "ejemplo.com",
        ] {
            assert!(Site::parse(input).is_err(), "should reject {input:?}");
        }
    }

    #[test]
    fn email_validates_minimal_shape() {
        assert!(validate_email(" persona@empresa.com ").is_ok());
        for bad in ["", "persona", "@empresa.com", "persona@empresa", "per sona@empresa.com", "a@.com"] {
            assert!(validate_email(bad).is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn empty_token_does_not_create_credentials() {
        let site = Site::parse("x.atlassian.net").unwrap();
        let r = Credentials::classic_token(site, "a@b.com", SecretString::from("   "));
        assert!(matches!(r, Err(AppError::EmptyToken)));
    }

    #[test]
    fn debug_does_not_show_the_token() {
        let site = Site::parse("x.atlassian.net").unwrap();
        let c = Credentials::classic_token(site, "a@b.com", SecretString::from("super-secret")).unwrap();
        let text = format!("{c:?}");
        assert!(!text.contains("super-secret"));
        assert!(text.contains("[hidden]"));
    }

    #[test]
    fn signs_with_basic_auth() {
        let site = Site::parse("x.atlassian.net").unwrap();
        let c = Credentials::classic_token(site, "a@b.com", SecretString::from("tok")).unwrap();
        let req = c.sign(reqwest::Client::new().get(c.base_url().clone())).build().unwrap();
        let value = req.headers().get(reqwest::header::AUTHORIZATION).unwrap().to_str().unwrap();
        // base64("a@b.com:tok") = YUBiLmNvbTp0b2s=
        assert_eq!(value, "Basic YUBiLmNvbTp0b2s=");
        assert!(req.headers().get(reqwest::header::AUTHORIZATION).unwrap().is_sensitive());
    }
}
