pub mod almacen;
pub mod auth;
pub mod avisos;
pub mod cache;
pub mod comandos;
pub mod config;
pub mod dominio;
pub mod error;
pub mod jira;
pub mod preferencias;
pub mod resumen;
pub mod sesion;

use tauri::Manager;

use crate::almacen::{AlmacenMemoria, AlmacenSecretos, AlmacenSistema};
use crate::sesion::Estado;

/// `SIGNAL_SIMULADO=1`/`0` manda; sin definir: simulado en debug y real en release.
fn modo_simulado() -> bool {
    match std::env::var("SIGNAL_SIMULADO").ok().as_deref().map(str::trim) {
        Some("1") => true,
        Some("0") => false,
        _ => cfg!(debug_assertions),
    }
}

pub fn run() {
    let simulado = modo_simulado();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let dir = app.path().app_config_dir()?;
            let dir_datos = app.path().app_local_data_dir()?;
            let almacen: Box<dyn AlmacenSecretos> =
                if simulado { Box::new(AlmacenMemoria::default()) } else { Box::new(AlmacenSistema) };
            app.manage(Estado::nuevo(simulado, dir, dir_datos, almacen));
            if simulado {
                if let Some(ventana) = app.get_webview_window("main") {
                    ventana.set_title("Signal Aduana — modo simulado")?;
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            comandos::restaurar_sesion,
            comandos::iniciar_sesion,
            comandos::cerrar_sesion,
            comandos::cargar_tareas,
            comandos::claves_con_horas,
            comandos::claves_con_pr,
            comandos::leer_horas,
            comandos::buscar_documentos,
            comandos::leer_cache,
            comandos::guardar_cache,
            comandos::calcular_vista,
            comandos::leer_preferencias,
            comandos::guardar_preferencias,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar Signal Aduana");
}
