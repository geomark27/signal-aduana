pub mod auth;
pub mod autostart;
pub mod cache;
pub mod commands;
pub mod config;
pub mod desktop;
pub mod diagnostics;
pub mod domain;
pub mod error;
pub mod jira;
pub mod monitor;
pub mod notifications;
pub mod preferences;
pub mod secret_store;
pub mod session;
pub mod summary;
pub mod updater;
pub mod watcher;

use tauri::Manager;

use crate::secret_store::{MemorySecretStore, SecretStore, SystemSecretStore};
use crate::session::AppState;

/// `SIGNAL_SIMULADO=1`/`0` rules; if undefined: simulated in debug and real in release.
fn simulated_mode() -> bool {
    match std::env::var("SIGNAL_SIMULADO").ok().as_deref().map(str::trim) {
        Some("1") => true,
        Some("0") => false,
        _ => cfg!(debug_assertions),
    }
}

pub fn run() {
    let simulated = simulated_mode();
    tauri::Builder::default()
        .plugin(desktop::single_instance())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![desktop::HIDDEN_ARG]),
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().app_local_data_dir()?;
            let secrets: Box<dyn SecretStore> =
                if simulated { Box::new(MemorySecretStore::default()) } else { Box::new(SystemSecretStore) };
            app.manage(AppState::new(simulated, config_dir, data_dir, secrets));
            if simulated {
                if let Some(window) = app.get_webview_window("main") {
                    window.set_title("Signal Aduana — modo simulado")?;
                }
            }
            app.manage(watcher::Watcher::new());
            desktop::setup(app)?;
            let state = app.state::<AppState>();
            let prefs = preferences::load(&state.config_dir, state.simulated);
            autostart::apply(app.handle(), &state, prefs);
            watcher::start(app.handle());
            if !simulated {
                updater::start(app.handle());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::restore_session,
            commands::sign_in,
            commands::sign_out,
            commands::load_issues,
            commands::keys_with_hours,
            commands::keys_with_pr,
            commands::read_worklogs,
            commands::find_documents,
            commands::read_cache,
            commands::save_cache,
            commands::compute_view,
            commands::read_preferences,
            commands::save_preferences,
            commands::build_diagnostics,
            updater::check_update,
            updater::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error starting Signal Aduana");
}
