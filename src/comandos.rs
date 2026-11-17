//! Comandos de Tauri (Contrato B). Cada uno es una capa fina sobre `session` y `jira`.
//! Si un comando de datos recibe `Unauthorized`, la sesión en memoria se descarta.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use serde_json::Value;
use tauri::{AppHandle, State};

use crate::cache::{self, Snapshot};
use crate::domain::{self, View, ViewInput};
use crate::error::{AppError, AppResult};
use crate::escritorio;
use crate::inicio;
use crate::jira::model::Issue;
use crate::jira::{self, WorklogsResult};
use crate::monitor;
use crate::preferencias::{self, Preferencias};
use crate::session::{self, AppState, Session, SessionInfo};
use crate::vigilancia;

/// Ejecuta una operación con la sesión activa y limpia la sesión si Jira rechaza el token.
async fn con_sesion<T, F, Fut>(estado: &AppState, f: F) -> AppResult<T>
where
    F: FnOnce(Arc<Session>) -> Fut,
    Fut: Future<Output = AppResult<T>>,
{
    let s = estado.current().await?;
    let r = f(s).await;
    if matches!(r, Err(AppError::Unauthorized)) {
        estado.clear().await;
    }
    r
}

#[tauri::command]
pub async fn restaurar_sesion(app: AppHandle, estado: State<'_, AppState>) -> AppResult<SessionInfo> {
    let info = session::restore(&estado).await;
    if info.connected {
        vigilancia::pedir_ciclo(&app);
    }
    Ok(info)
}

#[tauri::command]
pub async fn iniciar_sesion(
    app: AppHandle,
    estado: State<'_, AppState>,
    sitio: String,
    correo: String,
    token: String,
) -> AppResult<SessionInfo> {
    let info = session::sign_in(&estado, &sitio, &correo, &token).await?;
    vigilancia::pedir_ciclo(&app);
    Ok(info)
}

#[tauri::command]
pub async fn cerrar_sesion(app: AppHandle, estado: State<'_, AppState>) -> AppResult<SessionInfo> {
    let info = session::sign_out(&estado).await;
    escritorio::fijar_tooltip(&app, &monitor::texto_sin_sesion(estado.simulated));
    Ok(info)
}

#[tauri::command]
pub async fn cargar_tareas(estado: State<'_, AppState>) -> AppResult<Vec<Issue>> {
    con_sesion(&estado, |s| async move {
        jira::load_issues(&s.source, Some(&s.user.account_id)).await
    })
    .await
}

#[tauri::command]
pub async fn claves_con_horas(estado: State<'_, AppState>, desde: String) -> AppResult<Vec<String>> {
    con_sesion(&estado, |s| async move { jira::keys_with_hours(&s.source, &desde).await }).await
}

#[tauri::command]
pub async fn claves_con_pr(estado: State<'_, AppState>) -> AppResult<Vec<String>> {
    con_sesion(&estado, |s| async move { jira::keys_with_pr(&s.source).await }).await
}

#[tauri::command]
pub async fn leer_horas(
    estado: State<'_, AppState>,
    claves: Vec<String>,
    desde: i64,
) -> AppResult<WorklogsResult> {
    con_sesion(&estado, |s| async move {
        jira::read_worklogs(&s.source, &claves, desde, Some(&s.user.account_id)).await
    })
    .await
}

#[tauri::command]
pub async fn buscar_documentos(
    estado: State<'_, AppState>,
    claves: Vec<String>,
) -> AppResult<HashMap<String, String>> {
    con_sesion(&estado, |s| async move { jira::find_documents(&s.source, &claves).await }).await
}

/// Última carga guardada de la cuenta activa, o `null`.
#[tauri::command]
pub async fn leer_cache(estado: State<'_, AppState>) -> AppResult<Option<Snapshot>> {
    leer_cache_de(&estado).await
}

#[tauri::command]
pub async fn guardar_cache(estado: State<'_, AppState>, datos: Value) -> AppResult<()> {
    guardar_cache_de(&estado, datos).await
}

/// Calcula todo el tablero de una semana a partir de las tareas recibidas. No requiere sesión.
#[tauri::command]
pub fn calcular_vista(entrada: ViewInput) -> AppResult<View> {
    let hoy_ecuador = crate::jira::simulated::today_ecuador().format("%Y-%m-%d").to_string();
    domain::compute_view(entrada, &hoy_ecuador)
}

/// Preferencias del equipo (valores por defecto si no hay archivo o está dañado). No requiere sesión.
#[tauri::command]
pub fn leer_preferencias(estado: State<'_, AppState>) -> Preferencias {
    preferencias::cargar(&estado.config_dir, estado.simulated)
}

/// Normaliza y guarda las preferencias; devuelve las que quedaron guardadas. No requiere sesión.
#[tauri::command]
pub fn guardar_preferencias(
    app: AppHandle,
    estado: State<'_, AppState>,
    preferencias: Preferencias,
) -> AppResult<Preferencias> {
    let anteriores = preferencias::cargar(&estado.config_dir, estado.simulated);
    let p = preferencias::guardar(&estado.config_dir, estado.simulated, &preferencias)?;
    let guardadas = inicio::aplicar(&app, &estado, p);
    if anteriores.afecta_al_monitor(&guardadas) {
        vigilancia::pedir_ciclo(&app);
    }
    Ok(guardadas)
}

async fn leer_cache_de(estado: &AppState) -> AppResult<Option<Snapshot>> {
    let s = estado.current().await?;
    Ok(cache::read(&estado.data_dir, estado.simulated, &s.user.account_id, &s.site.as_text()))
}

async fn guardar_cache_de(estado: &AppState, datos: Value) -> AppResult<()> {
    let s = estado.current().await?;
    cache::save(&estado.data_dir, estado.simulated, &s.user.account_id, &s.site.as_text(), datos)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::secret_store::MemorySecretStore;

    #[tokio::test]
    async fn sin_sesion_y_no_autorizado_limpia() {
        let dir = tempfile::tempdir().unwrap();
        let e = AppState::new(true, dir.path().to_path_buf(), dir.path().join("datos"), Box::new(MemorySecretStore::default()));
        let r = con_sesion(&e, |_| async { Ok(()) }).await;
        assert!(matches!(r, Err(AppError::NoSession)));

        session::sign_in(&e, "https://x.atlassian.net", "a@b.com", "t").await.unwrap();
        let r: AppResult<()> = con_sesion(&e, |_| async { Err(AppError::Unauthorized) }).await;
        assert!(matches!(r, Err(AppError::Unauthorized)));
        assert!(matches!(e.current().await, Err(AppError::NoSession)));
    }

    #[tokio::test]
    async fn cache_requiere_sesion_y_hace_ida_y_vuelta() {
        let dir = tempfile::tempdir().unwrap();
        let e = AppState::new(true, dir.path().to_path_buf(), dir.path().join("datos"), Box::new(MemorySecretStore::default()));
        assert!(matches!(leer_cache_de(&e).await, Err(AppError::NoSession)));
        assert!(matches!(guardar_cache_de(&e, serde_json::json!(1)).await, Err(AppError::NoSession)));

        session::sign_in(&e, "https://x.atlassian.net", "a@b.com", "t").await.unwrap();
        assert!(leer_cache_de(&e).await.unwrap().is_none());
        guardar_cache_de(&e, serde_json::json!({ "format": 1 })).await.unwrap();
        let i = leer_cache_de(&e).await.unwrap().unwrap();
        assert_eq!(i.data, serde_json::json!({ "format": 1 }));
    }
}
