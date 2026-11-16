//! Integración con el escritorio: ícono en la bandeja, cierre a la bandeja, una sola instancia
//! y arranque oculto (`--oculto`).

use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry, WindowEvent};

use crate::preferencias;
use crate::resumen;
use crate::sesion::Estado;

/// Identificador del ícono de la bandeja.
pub const ID_BANDEJA: &str = "bandeja";
/// Etiqueta de la ventana principal.
pub const VENTANA: &str = "main";
/// Argumento que pone el inicio con Windows para arrancar con la ventana escondida.
pub const ARG_OCULTO: &str = "--oculto";
/// Evento que pide a la interfaz actualizar los datos.
pub const EVENTO_ACTUALIZAR: &str = "signal://actualizar";

/// `true` si algún argumento es exactamente `--oculto`.
pub fn es_arranque_oculto(args: &[String]) -> bool {
    args.iter().any(|a| a == ARG_OCULTO)
}

/// Plugin de una sola instancia: un segundo arranque trae al frente la ventana existente.
pub fn instancia_unica() -> tauri::plugin::TauriPlugin<Wry> {
    tauri_plugin_single_instance::init(|app, _args, _cwd| mostrar_ventana(app))
}

/// Muestra, restaura y enfoca la ventana principal.
pub fn mostrar_ventana(app: &AppHandle) {
    let Some(ventana) = app.get_webview_window(VENTANA) else {
        eprintln!("escritorio: no existe la ventana principal");
        return;
    };
    if let Err(e) = ventana.show() {
        eprintln!("escritorio: no se pudo mostrar la ventana: {e}");
    }
    if let Err(e) = ventana.unminimize() {
        eprintln!("escritorio: no se pudo restaurar la ventana: {e}");
    }
    if let Err(e) = ventana.set_focus() {
        eprintln!("escritorio: no se pudo enfocar la ventana: {e}");
    }
}

/// Cambia el tooltip del ícono de la bandeja.
pub fn fijar_tooltip(app: &AppHandle, texto: &str) {
    let Some(bandeja) = app.tray_by_id(ID_BANDEJA) else {
        eprintln!("escritorio: no existe el ícono de la bandeja");
        return;
    };
    if let Err(e) = bandeja.set_tooltip(Some(texto)) {
        eprintln!("escritorio: no se pudo cambiar el tooltip: {e}");
    }
}

fn debe_mostrar_al_arrancar(oculto: bool, bandeja_ok: bool) -> bool {
    !oculto || !bandeja_ok
}

fn debe_esconder_al_cerrar(minimizar_a_bandeja: bool, bandeja_ok: bool) -> bool {
    minimizar_a_bandeja && bandeja_ok
}

fn crear_bandeja(app: &tauri::App) -> tauri::Result<()> {
    let simulado = app.state::<Estado>().simulado;

    let abrir = MenuItemBuilder::with_id("abrir", "Abrir Signal Aduana").build(app)?;
    let actualizar = MenuItemBuilder::with_id("actualizar", "Actualizar ahora").build(app)?;
    let separador = PredefinedMenuItem::separator(app)?;
    let salir = MenuItemBuilder::with_id("salir", "Salir").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&abrir, &actualizar, &separador, &salir])
        .build()?;

    let mut bandeja = TrayIconBuilder::with_id(ID_BANDEJA)
        .tooltip(resumen::texto_bandeja(None, simulado))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, evento| match evento.id().as_ref() {
            "abrir" => mostrar_ventana(app),
            "actualizar" => {
                if let Err(e) = app.emit(EVENTO_ACTUALIZAR, ()) {
                    eprintln!("escritorio: no se pudo emitir la actualización: {e}");
                }
                crate::vigilancia::pedir_ciclo(app);
            }
            "salir" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|bandeja, evento| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = evento
            {
                mostrar_ventana(bandeja.app_handle());
            }
        });
    if let Some(icono) = app.default_window_icon() {
        bandeja = bandeja.icon(icono.clone());
    }
    bandeja.build(app)?;
    Ok(())
}

/// Crea la bandeja y su menú, engancha el cierre de la ventana y aplica el arranque oculto.
/// Si la bandeja no se puede crear, la ventana se muestra y cerrarla cierra la app.
pub fn configurar(app: &tauri::App) -> tauri::Result<()> {
    let bandeja_ok = match crear_bandeja(app) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("escritorio: no se pudo crear la bandeja: {e}");
            false
        }
    };

    if let Some(ventana) = app.get_webview_window(VENTANA) {
        let app_handle = app.handle().clone();
        let ventana_cierre = ventana.clone();
        ventana.on_window_event(move |evento| {
            if let WindowEvent::CloseRequested { api, .. } = evento {
                let estado = app_handle.state::<Estado>();
                let prefs = preferencias::cargar(&estado.dir_config, estado.simulado);
                if debe_esconder_al_cerrar(prefs.minimizar_a_bandeja, bandeja_ok) {
                    api.prevent_close();
                    if let Err(e) = ventana_cierre.hide() {
                        eprintln!("escritorio: no se pudo esconder la ventana: {e}");
                    }
                }
            }
        });

        let args: Vec<String> = std::env::args().collect();
        if debe_mostrar_al_arrancar(es_arranque_oculto(&args), bandeja_ok) {
            if let Err(e) = ventana.show() {
                eprintln!("escritorio: no se pudo mostrar la ventana: {e}");
            }
            if let Err(e) = ventana.set_focus() {
                eprintln!("escritorio: no se pudo enfocar la ventana: {e}");
            }
        }
    } else {
        eprintln!("escritorio: no existe la ventana principal");
    }
    Ok(())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn detecta_oculto_entre_otros_argumentos() {
        assert!(es_arranque_oculto(&args(&["signal-aduana.exe", "--oculto", "--otro"])));
        assert!(es_arranque_oculto(&args(&["--oculto"])));
    }

    #[test]
    fn sin_argumentos_no_es_oculto() {
        assert!(!es_arranque_oculto(&args(&["signal-aduana.exe"])));
        assert!(!es_arranque_oculto(&[]));
    }

    #[test]
    fn mostrar_al_arrancar_tabla_de_verdad() {
        assert!(debe_mostrar_al_arrancar(false, true));
        assert!(debe_mostrar_al_arrancar(false, false));
        assert!(!debe_mostrar_al_arrancar(true, true));
        assert!(debe_mostrar_al_arrancar(true, false));
    }

    #[test]
    fn esconder_al_cerrar_tabla_de_verdad() {
        assert!(debe_esconder_al_cerrar(true, true));
        assert!(!debe_esconder_al_cerrar(true, false));
        assert!(!debe_esconder_al_cerrar(false, true));
        assert!(!debe_esconder_al_cerrar(false, false));
    }

    #[test]
    fn no_cuentan_variantes() {
        assert!(!es_arranque_oculto(&args(&["--ocultos"])));
        assert!(!es_arranque_oculto(&args(&["oculto"])));
        assert!(!es_arranque_oculto(&args(&["--OCULTO", "-oculto"])));
    }
}
