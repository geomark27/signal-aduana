//! Jira session: sign in, restore and sign out. Tauri-free logic, so it can be tested.
//!
//! The token only lives in the secret store and inside the data source; it is never
//! logged or returned to the UI.

use std::path::PathBuf;
use std::sync::Arc;

use secrecy::SecretString;
use serde::Serialize;
use tokio::sync::RwLock;

use crate::auth::{validate_email, Credentials, Site};
use crate::cache;
use crate::config::{self, Config};
use crate::error::{AppError, AppResult};
use crate::jira::http::JiraHttp;
use crate::jira::model::User;
use crate::jira::simulated::{today_ecuador, SimulatedJira};
use crate::jira::Source;
use crate::secret_store::{service_for, SecretStore};

/// Token that, in simulated mode, makes Jira «reject» the credentials.
const REJECTED_TOKEN: &str = "rechazar";

pub struct Session {
    pub source: Source,
    pub user: User,
    pub site: Site,
    pub email: String,
}

pub struct AppState {
    pub simulated: bool,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub secrets: Box<dyn SecretStore>,
    pub session: RwLock<Option<Arc<Session>>>,
}

/// What the UI knows about the session (Contract B).
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct SessionInfo {
    pub connected: bool,
    pub simulated: bool,
    pub user: Option<User>,
    pub site: Option<String>,
    pub email: Option<String>,
    pub notice: Option<String>,
}

impl AppState {
    pub fn new(
        simulated: bool,
        config_dir: PathBuf,
        data_dir: PathBuf,
        secrets: Box<dyn SecretStore>,
    ) -> Self {
        AppState { simulated, config_dir, data_dir, secrets, session: RwLock::new(None) }
    }

    fn config_path(&self) -> PathBuf {
        config::path(&self.config_dir, self.simulated)
    }

    /// Active session or `NoSession`.
    pub async fn current(&self) -> AppResult<Arc<Session>> {
        self.session.read().await.clone().ok_or(AppError::NoSession)
    }

    /// Drops the in-memory session (does not touch the saved token).
    pub async fn clear(&self) {
        *self.session.write().await = None;
    }

    fn create_source(&self, cred: Credentials, email: &str, token: &str) -> AppResult<Source> {
        if self.simulated {
            Ok(Source::Simulated(SimulatedJira::new(email, token != REJECTED_TOKEN, today_ecuador())))
        } else {
            Ok(Source::Http(JiraHttp::new(cred)?))
        }
    }

    fn info(&self, s: &Session, notice: Option<String>) -> SessionInfo {
        SessionInfo {
            connected: true,
            simulated: self.simulated,
            user: Some(s.user.clone()),
            site: Some(s.site.as_text()),
            email: Some(s.email.clone()),
            notice,
        }
    }

    fn saved_info(&self, notice: Option<String>) -> SessionInfo {
        let c = Config::load(&self.config_path());
        SessionInfo { connected: false, simulated: self.simulated, user: None, site: c.site, email: c.email, notice }
    }
}

pub async fn sign_in(state: &AppState, site: &str, email: &str, token: &str) -> AppResult<SessionInfo> {
    let site = Site::parse(site)?;
    let email = validate_email(email)?;
    let secret = SecretString::from(token.to_string());
    let cred = Credentials::classic_token(site.clone(), &email, secret.clone())?;
    let source = state.create_source(cred, &email, token)?;
    let user = source.myself().await?;

    // The connection already worked: if saving fails, the session continues and a notice is shown.
    let mut notice = None;
    if let Err(e) = state.secrets.save(&service_for(site.host()), &email, &secret) {
        notice = Some(format!("Conectado, pero no se pudo guardar el token para la próxima vez. {e}"));
    }
    let cfg = Config { site: Some(site.as_text()), email: Some(email.clone()) };
    if let Err(e) = cfg.save(&state.config_path()) {
        notice.get_or_insert_with(|| format!("Conectado, pero no se pudo guardar la configuración. {e}"));
    }

    let s = Arc::new(Session { source, user, site, email });
    let info = state.info(&s, notice);
    *state.session.write().await = Some(s);
    Ok(info)
}

pub async fn restore(state: &AppState) -> SessionInfo {
    if let Some(s) = state.session.read().await.clone() {
        return state.info(&s, None);
    }
    let cfg = Config::load(&state.config_path());
    let (Some(site_text), Some(email)) = (cfg.site.clone(), cfg.email.clone()) else {
        return state.saved_info(None);
    };
    let Ok(site) = Site::parse(&site_text) else {
        return state.saved_info(None);
    };
    let token = match state.secrets.read(&service_for(site.host()), &email) {
        Ok(Some(t)) => t,
        Ok(None) => return state.saved_info(None),
        Err(e) => return state.saved_info(Some(format!("No se pudo leer el token guardado. {e}"))),
    };
    use secrecy::ExposeSecret;
    let token_text = token.expose_secret().to_string();
    let attempt = async {
        let cred = Credentials::classic_token(site.clone(), &email, token)?;
        let source = state.create_source(cred, &email, &token_text)?;
        let user = source.myself().await?;
        Ok::<_, AppError>(Session { source, user, site: site.clone(), email: email.clone() })
    };
    match attempt.await {
        Ok(s) => {
            let s = Arc::new(s);
            let info = state.info(&s, None);
            *state.session.write().await = Some(s);
            info
        }
        Err(AppError::Unauthorized) => state.saved_info(Some(
            "Tu token guardado ya no es válido (venció o fue revocado). Genera uno nuevo y vuelve a conectarte.".into(),
        )),
        Err(e) => state.saved_info(Some(format!("No se pudo restaurar la sesión. {e}"))),
    }
}

pub async fn sign_out(state: &AppState) -> SessionInfo {
    let cfg = Config::load(&state.config_path());
    let mut notice = None;
    if let (Some(site), Some(email)) = (&cfg.site, &cfg.email) {
        if let Ok(site) = Site::parse(site) {
            if let Err(e) = state.secrets.delete(&service_for(site.host()), email) {
                notice = Some(format!("Se cerró la sesión, pero no se pudo borrar el token guardado. {e}"));
            }
        }
    }
    cache::delete(&state.data_dir, state.simulated);
    state.clear().await;
    state.saved_info(notice)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret_store::MemorySecretStore;
    use secrecy::ExposeSecret;

    const SITE: &str = "https://prueba.atlassian.net";
    const EMAIL: &str = "persona@ejemplo.com";

    fn app_state(dir: &tempfile::TempDir) -> AppState {
        AppState::new(true, dir.path().to_path_buf(), dir.path().join("datos"), Box::new(MemorySecretStore::default()))
    }

    #[tokio::test]
    async fn sign_in_saves_token_and_config() {
        let dir = tempfile::tempdir().unwrap();
        let e = app_state(&dir);
        let info = sign_in(&e, "prueba.atlassian.net", EMAIL, "tok").await.unwrap();
        assert!(info.connected && info.simulated);
        assert_eq!(info.site.as_deref(), Some(SITE));
        assert_eq!(info.user.unwrap().account_id, "sim-0001");
        let t = e.secrets.read(&service_for("prueba.atlassian.net"), EMAIL).unwrap().unwrap();
        assert_eq!(t.expose_secret(), "tok");
        assert_eq!(Config::load(&e.config_path()).email.as_deref(), Some(EMAIL));
        assert!(e.current().await.is_ok());
    }

    #[tokio::test]
    async fn rejected_token_saves_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let e = app_state(&dir);
        let r = sign_in(&e, SITE, EMAIL, "rechazar").await;
        assert!(matches!(r, Err(AppError::Unauthorized)));
        assert!(e.secrets.read(&service_for("prueba.atlassian.net"), EMAIL).unwrap().is_none());
        assert!(matches!(e.current().await, Err(AppError::NoSession)));
    }

    #[tokio::test]
    async fn validates_inputs() {
        let dir = tempfile::tempdir().unwrap();
        let e = app_state(&dir);
        assert!(matches!(sign_in(&e, "https://google.com", EMAIL, "t").await, Err(AppError::InvalidSite(_))));
        assert!(matches!(sign_in(&e, SITE, "sin-arroba", "t").await, Err(AppError::InvalidEmail)));
        assert!(matches!(sign_in(&e, SITE, EMAIL, "  ").await, Err(AppError::EmptyToken)));
    }

    #[tokio::test]
    async fn restore_and_sign_out() {
        let dir = tempfile::tempdir().unwrap();
        let e = app_state(&dir);
        sign_in(&e, SITE, EMAIL, "tok").await.unwrap();
        e.clear().await;

        let info = restore(&e).await;
        assert!(info.connected);
        assert_eq!(info.email.as_deref(), Some(EMAIL));

        let info = sign_out(&e).await;
        assert!(!info.connected);
        // Keeps site and email to prefill the form, but the token is gone.
        assert_eq!(info.site.as_deref(), Some(SITE));
        assert!(e.current().await.is_err());
        let info = restore(&e).await;
        assert!(!info.connected);
        assert_eq!(info.email.as_deref(), Some(EMAIL));
    }

    #[tokio::test]
    async fn restore_with_expired_token_warns() {
        let dir = tempfile::tempdir().unwrap();
        let e = app_state(&dir);
        sign_in(&e, SITE, EMAIL, "tok").await.unwrap();
        e.clear().await;
        e.secrets
            .save(&service_for("prueba.atlassian.net"), EMAIL, &SecretString::from("rechazar"))
            .unwrap();
        let info = restore(&e).await;
        assert!(!info.connected);
        assert!(info.notice.unwrap().contains("ya no es válido"));
        assert_eq!(info.site.as_deref(), Some(SITE));
    }

    #[tokio::test]
    async fn sign_out_deletes_the_cache() {
        let dir = tempfile::tempdir().unwrap();
        let e = app_state(&dir);
        sign_in(&e, SITE, EMAIL, "tok").await.unwrap();
        cache::save(&e.data_dir, true, "sim-0001", SITE, serde_json::json!({ "a": 1 })).unwrap();
        assert!(cache::read(&e.data_dir, true, "sim-0001", SITE).is_some());
        sign_out(&e).await;
        assert!(cache::read(&e.data_dir, true, "sim-0001", SITE).is_none());
        assert!(!cache::path(&e.data_dir, true).exists());
    }

    #[tokio::test]
    async fn restore_without_configuration() {
        let dir = tempfile::tempdir().unwrap();
        let info = restore(&app_state(&dir)).await;
        assert_eq!(info, SessionInfo { simulated: true, ..Default::default() });
    }
}
