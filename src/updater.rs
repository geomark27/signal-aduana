//! Self-update: `check_update` and `install_update` commands plus a background check.
//! In simulated mode nothing touches the network and nothing is installed.

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::UpdaterExt;

use crate::error::{AppError, AppResult};
use crate::session::AppState;

/// Wait before the first background check.
pub const FIRST_CHECK: Duration = Duration::from_secs(60);
/// Time between background checks.
pub const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

pub const EVENT_AVAILABLE: &str = "signal://update-available";
pub const EVENT_PROGRESS: &str = "signal://update-progress";

/// New version offered by the update endpoint.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
}

/// Answer of `check_update`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub current: String,
    pub simulated: bool,
    pub update: Option<UpdateInfo>,
}

/// Notify only when `version` differs from the last one already notified.
pub fn should_notify(last: Option<&str>, version: &str) -> bool {
    last != Some(version)
}

fn update_error(e: tauri_plugin_updater::Error) -> AppError {
    AppError::Update(e.to_string())
}

async fn find_update(app: &AppHandle) -> AppResult<Option<UpdateInfo>> {
    let updater = app.updater().map_err(update_error)?;
    let update = updater.check().await.map_err(update_error)?;
    Ok(update.map(|u| UpdateInfo { version: u.version, notes: u.body }))
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> AppResult<UpdateCheck> {
    let current = app.package_info().version.to_string();
    let simulated = app.state::<AppState>().simulated;
    if simulated {
        return Ok(UpdateCheck { current, simulated, update: None });
    }
    let update = find_update(&app).await?;
    Ok(UpdateCheck { current, simulated, update })
}

/// Downloads, verifies the signature and installs; emits `signal://update-progress` with
/// `[downloaded, total | null]` and restarts the app when done.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> AppResult<()> {
    if app.state::<AppState>().simulated {
        return Err(AppError::Update("en modo simulado no se instalan actualizaciones".into()));
    }
    let updater = app.updater().map_err(update_error)?;
    let Some(update) = updater.check().await.map_err(update_error)? else {
        return Err(AppError::Update("ya tienes la última versión".into()));
    };
    let mut downloaded = 0u64;
    update
        .download_and_install(
            |chunk, total| {
                downloaded += chunk as u64;
                let _ = app.emit(EVENT_PROGRESS, (downloaded, total));
            },
            || {},
        )
        .await
        .map_err(update_error)?;
    // On Windows the installer already closed the app; elsewhere it must be relaunched.
    app.restart()
}

/// Launches the background check (not in simulated mode): first after 60 s, then every 6 h.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if app.state::<AppState>().simulated {
            return;
        }
        tokio::time::sleep(FIRST_CHECK).await;
        let mut notified: Option<String> = None;
        loop {
            match find_update(&app).await {
                Ok(Some(update)) => {
                    let _ = app.emit(EVENT_AVAILABLE, update.clone());
                    if should_notify(notified.as_deref(), &update.version) {
                        let shown = app
                            .notification()
                            .builder()
                            .title(format!("Signal Aduana {} disponible", update.version))
                            .body("Ábrela y pulsa «Actualizar ahora» en el aviso de arriba.")
                            .show();
                        if let Err(e) = shown {
                            eprintln!("updater: could not show the notification: {e}");
                        }
                        notified = Some(update.version);
                    }
                }
                Ok(None) => {}
                Err(e) => eprintln!("updater: background check failed: {e}"),
            }
            tokio::time::sleep(CHECK_EVERY).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notifies_once_per_version() {
        assert!(should_notify(None, "0.2.0"));
        assert!(!should_notify(Some("0.2.0"), "0.2.0"));
        assert!(should_notify(Some("0.2.0"), "0.3.0"));
    }

    #[test]
    fn check_update_json_without_update() {
        let check = UpdateCheck { current: "0.1.0".into(), simulated: true, update: None };
        let json = serde_json::to_value(&check).unwrap();
        assert_eq!(json.to_string(), r#"{"current":"0.1.0","simulated":true,"update":null}"#);
    }

    #[test]
    fn check_update_json_with_update() {
        let check = UpdateCheck {
            current: "0.1.0".into(),
            simulated: false,
            update: Some(UpdateInfo { version: "0.2.0".into(), notes: None }),
        };
        let json = serde_json::to_value(&check).unwrap();
        assert_eq!(
            json.to_string(),
            r#"{"current":"0.1.0","simulated":false,"update":{"notes":null,"version":"0.2.0"}}"#
        );
    }

    #[test]
    fn update_error_code_and_message() {
        let json = serde_json::to_value(AppError::Update("ya tienes la última versión".into())).unwrap();
        assert_eq!(json["code"], "update");
        assert_eq!(json["message"], "No se pudo actualizar: ya tienes la última versión");
    }

    #[test]
    fn fixed_times() {
        assert_eq!(FIRST_CHECK.as_secs(), 60);
        assert_eq!(CHECK_EVERY.as_secs(), 21600);
    }
}
