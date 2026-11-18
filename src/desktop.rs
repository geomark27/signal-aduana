//! Desktop integration: tray icon, close to tray, single instance and hidden start (`--hidden`).

use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry, WindowEvent};

use crate::preferences;
use crate::session::AppState;
use crate::summary;

/// Tray icon identifier.
pub const TRAY_ID: &str = "tray";
/// Label of the main window.
pub const MAIN_WINDOW: &str = "main";
/// Argument that the Windows startup adds to start with the window hidden.
pub const HIDDEN_ARG: &str = "--hidden";
/// Event that asks the interface to refresh the data.
pub const REFRESH_EVENT: &str = "signal://refresh";

/// `true` if any argument is exactly `--hidden`.
pub fn is_hidden_start(args: &[String]) -> bool {
    args.iter().any(|a| a == HIDDEN_ARG)
}

/// Single-instance plugin: a second launch brings the existing window to the front.
pub fn single_instance() -> tauri::plugin::TauriPlugin<Wry> {
    tauri_plugin_single_instance::init(|app, _args, _cwd| show_window(app))
}

/// Shows, restores and focuses the main window.
pub fn show_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        eprintln!("desktop: the main window does not exist");
        return;
    };
    if let Err(e) = window.show() {
        eprintln!("desktop: could not show the window: {e}");
    }
    if let Err(e) = window.unminimize() {
        eprintln!("desktop: could not restore the window: {e}");
    }
    if let Err(e) = window.set_focus() {
        eprintln!("desktop: could not focus the window: {e}");
    }
}

/// Changes the tooltip of the tray icon.
pub fn set_tooltip(app: &AppHandle, text: &str) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        eprintln!("desktop: the tray icon does not exist");
        return;
    };
    if let Err(e) = tray.set_tooltip(Some(text)) {
        eprintln!("desktop: could not change the tooltip: {e}");
    }
}

fn should_show_on_start(hidden: bool, tray_ok: bool) -> bool {
    !hidden || !tray_ok
}

fn should_hide_on_close(minimize_to_tray: bool, tray_ok: bool) -> bool {
    minimize_to_tray && tray_ok
}

fn create_tray(app: &tauri::App) -> tauri::Result<()> {
    let simulated = app.state::<AppState>().simulated;

    let open = MenuItemBuilder::with_id("open", "Abrir Signal Aduana").build(app)?;
    let refresh = MenuItemBuilder::with_id("refresh", "Actualizar ahora").build(app)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Salir").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&open, &refresh, &separator, &quit])
        .build()?;

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(summary::tray_text(None, simulated))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_window(app),
            "refresh" => {
                if let Err(e) = app.emit(REFRESH_EVENT, ()) {
                    eprintln!("desktop: could not emit the refresh: {e}");
                }
                crate::watcher::request_cycle(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Creates the tray and its menu, hooks the window close and applies the hidden start.
/// If the tray cannot be created, the window is shown and closing it closes the app.
pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let tray_ok = match create_tray(app) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("desktop: could not create the tray: {e}");
            false
        }
    };

    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let app_handle = app.handle().clone();
        let close_window = window.clone();
        window.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = app_handle.state::<AppState>();
                let prefs = preferences::load(&state.config_dir, state.simulated);
                if should_hide_on_close(prefs.minimize_to_tray, tray_ok) {
                    api.prevent_close();
                    if let Err(e) = close_window.hide() {
                        eprintln!("desktop: could not hide the window: {e}");
                    }
                }
            }
        });

        let args: Vec<String> = std::env::args().collect();
        if should_show_on_start(is_hidden_start(&args), tray_ok) {
            if let Err(e) = window.show() {
                eprintln!("desktop: could not show the window: {e}");
            }
            if let Err(e) = window.set_focus() {
                eprintln!("desktop: could not focus the window: {e}");
            }
        }
    } else {
        eprintln!("desktop: the main window does not exist");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn detects_hidden_among_other_arguments() {
        assert!(is_hidden_start(&args(&["signal-aduana.exe", "--hidden", "--other"])));
        assert!(is_hidden_start(&args(&["--hidden"])));
    }

    #[test]
    fn no_arguments_is_not_hidden() {
        assert!(!is_hidden_start(&args(&["signal-aduana.exe"])));
        assert!(!is_hidden_start(&[]));
    }

    #[test]
    fn show_on_start_truth_table() {
        assert!(should_show_on_start(false, true));
        assert!(should_show_on_start(false, false));
        assert!(!should_show_on_start(true, true));
        assert!(should_show_on_start(true, false));
    }

    #[test]
    fn hide_on_close_truth_table() {
        assert!(should_hide_on_close(true, true));
        assert!(!should_hide_on_close(true, false));
        assert!(!should_hide_on_close(false, true));
        assert!(!should_hide_on_close(false, false));
    }

    #[test]
    fn variants_do_not_count() {
        assert!(!is_hidden_start(&args(&["--hiddens"])));
        assert!(!is_hidden_start(&args(&["hidden"])));
        assert!(!is_hidden_start(&args(&["--HIDDEN", "-hidden"])));
    }
}
