//! Tauri commands (Contract B). Each one is a thin layer over `session` and `jira`.
//! If a data command receives `Unauthorized`, the in-memory session is discarded.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use serde_json::Value;
use tauri::{AppHandle, State};

use crate::autostart;
use crate::cache::{self, Snapshot};
use crate::desktop;
use crate::diagnostics;
use crate::domain::{self, View, ViewInput};
use crate::error::{AppError, AppResult};
use crate::jira::model::Issue;
use crate::jira::{self, WorklogsResult};
use crate::monitor;
use crate::preferences::{self, Preferences};
use crate::session::{self, AppState, Session, SessionInfo};
use crate::watcher;

/// Runs an operation with the active session and clears the session if Jira rejects the token.
async fn with_session<T, F, Fut>(state: &AppState, f: F) -> AppResult<T>
where
    F: FnOnce(Arc<Session>) -> Fut,
    Fut: Future<Output = AppResult<T>>,
{
    let s = state.current().await?;
    let r = f(s).await;
    if matches!(r, Err(AppError::Unauthorized)) {
        state.clear().await;
    }
    r
}

#[tauri::command]
pub async fn restore_session(app: AppHandle, state: State<'_, AppState>) -> AppResult<SessionInfo> {
    let info = session::restore(&state).await;
    if info.connected {
        watcher::request_cycle(&app);
    }
    Ok(info)
}

#[tauri::command]
pub async fn sign_in(
    app: AppHandle,
    state: State<'_, AppState>,
    site: String,
    email: String,
    token: String,
) -> AppResult<SessionInfo> {
    let info = session::sign_in(&state, &site, &email, &token).await?;
    watcher::request_cycle(&app);
    Ok(info)
}

#[tauri::command]
pub async fn sign_out(app: AppHandle, state: State<'_, AppState>) -> AppResult<SessionInfo> {
    let info = session::sign_out(&state).await;
    desktop::set_tooltip(&app, &monitor::no_session_text(state.simulated));
    Ok(info)
}

#[tauri::command]
pub async fn load_issues(state: State<'_, AppState>) -> AppResult<Vec<Issue>> {
    with_session(&state, |s| async move {
        jira::load_issues(&s.source, Some(&s.user.account_id)).await
    })
    .await
}

#[tauri::command]
pub async fn keys_with_hours(state: State<'_, AppState>, since: String) -> AppResult<Vec<String>> {
    with_session(&state, |s| async move { jira::keys_with_hours(&s.source, &since).await }).await
}

#[tauri::command]
pub async fn keys_with_pr(state: State<'_, AppState>) -> AppResult<Vec<String>> {
    with_session(&state, |s| async move { jira::keys_with_pr(&s.source).await }).await
}

#[tauri::command]
pub async fn read_worklogs(
    state: State<'_, AppState>,
    keys: Vec<String>,
    since: i64,
) -> AppResult<WorklogsResult> {
    with_session(&state, |s| async move {
        jira::read_worklogs(&s.source, &keys, since, Some(&s.user.account_id)).await
    })
    .await
}

#[tauri::command]
pub async fn find_documents(
    state: State<'_, AppState>,
    keys: Vec<String>,
) -> AppResult<HashMap<String, String>> {
    with_session(&state, |s| async move { jira::find_documents(&s.source, &keys).await }).await
}

/// Last saved load of the active account, or `null`.
#[tauri::command]
pub async fn read_cache(state: State<'_, AppState>) -> AppResult<Option<Snapshot>> {
    read_cache_for(&state).await
}

#[tauri::command]
pub async fn save_cache(state: State<'_, AppState>, data: Value) -> AppResult<()> {
    save_cache_for(&state, data).await
}

/// Computes the whole board of a week from the received issues. Does not require a session.
#[tauri::command]
pub fn compute_view(input: ViewInput) -> AppResult<View> {
    let today_ecuador = crate::jira::simulated::today_ecuador().format("%Y-%m-%d").to_string();
    domain::compute_view(input, &today_ecuador)
}

/// Team preferences (defaults if there is no file or it is corrupt). Does not require a session.
#[tauri::command]
pub fn read_preferences(state: State<'_, AppState>) -> Preferences {
    preferences::load(&state.config_dir, state.simulated)
}

/// Normalizes and saves the preferences; returns the ones that were saved. Does not require a session.
#[tauri::command]
pub fn save_preferences(
    app: AppHandle,
    state: State<'_, AppState>,
    preferences: Preferences,
) -> AppResult<Preferences> {
    let previous = preferences::load(&state.config_dir, state.simulated);
    let p = preferences::save(&state.config_dir, state.simulated, &preferences)?;
    let saved = autostart::apply(&app, &state, p);
    if previous.affects_monitor(&saved) {
        watcher::request_cycle(&app);
    }
    Ok(saved)
}

/// Anonymous diagnostics (JSON text) of a load walk against the active source.
#[tauri::command]
pub async fn build_diagnostics(state: State<'_, AppState>) -> AppResult<String> {
    build_diagnostics_for(&state).await
}

async fn build_diagnostics_for(state: &AppState) -> AppResult<String> {
    let simulated = state.simulated;
    with_session(state, |s| async move {
        let facts = diagnostics::SessionFacts {
            site: s.site.as_text(),
            email: s.email.clone(),
            name: s.user.name.clone(),
            account_id: s.user.account_id.clone(),
        };
        let report = diagnostics::build(&s.source, &facts, simulated, crate::jira::simulated::today_ecuador()).await?;
        Ok(diagnostics::to_text(&report))
    })
    .await
}

async fn read_cache_for(state: &AppState) -> AppResult<Option<Snapshot>> {
    let s = state.current().await?;
    Ok(cache::read(&state.data_dir, state.simulated, &s.user.account_id, &s.site.as_text()))
}

async fn save_cache_for(state: &AppState, data: Value) -> AppResult<()> {
    let s = state.current().await?;
    cache::save(&state.data_dir, state.simulated, &s.user.account_id, &s.site.as_text(), data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret_store::MemorySecretStore;

    #[tokio::test]
    async fn no_session_and_unauthorized_clears() {
        let dir = tempfile::tempdir().unwrap();
        let e = AppState::new(true, dir.path().to_path_buf(), dir.path().join("data"), Box::new(MemorySecretStore::default()));
        let r = with_session(&e, |_| async { Ok(()) }).await;
        assert!(matches!(r, Err(AppError::NoSession)));

        session::sign_in(&e, "https://x.atlassian.net", "a@b.com", "t").await.unwrap();
        let r: AppResult<()> = with_session(&e, |_| async { Err(AppError::Unauthorized) }).await;
        assert!(matches!(r, Err(AppError::Unauthorized)));
        assert!(matches!(e.current().await, Err(AppError::NoSession)));
    }

    #[tokio::test]
    async fn cache_requires_session_and_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let e = AppState::new(true, dir.path().to_path_buf(), dir.path().join("data"), Box::new(MemorySecretStore::default()));
        assert!(matches!(read_cache_for(&e).await, Err(AppError::NoSession)));
        assert!(matches!(save_cache_for(&e, serde_json::json!(1)).await, Err(AppError::NoSession)));

        session::sign_in(&e, "https://x.atlassian.net", "a@b.com", "t").await.unwrap();
        assert!(read_cache_for(&e).await.unwrap().is_none());
        save_cache_for(&e, serde_json::json!({ "format": 1 })).await.unwrap();
        let i = read_cache_for(&e).await.unwrap().unwrap();
        assert_eq!(i.data, serde_json::json!({ "format": 1 }));
    }

    #[tokio::test]
    async fn diagnostics_require_session_and_return_json() {
        let dir = tempfile::tempdir().unwrap();
        let e = AppState::new(true, dir.path().to_path_buf(), dir.path().join("data"), Box::new(MemorySecretStore::default()));
        assert!(matches!(build_diagnostics_for(&e).await, Err(AppError::NoSession)));

        session::sign_in(&e, "https://x.atlassian.net", "a@b.com", "t").await.unwrap();
        let text = build_diagnostics_for(&e).await.unwrap();
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["app"]["mode"], "simulated");
        assert!(!text.contains("atlassian.net"));
    }
}
