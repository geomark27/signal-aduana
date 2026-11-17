//! Inicio con Windows según la preferencia `iniciarConWindows`. En modo simulado nunca toca el
//! sistema: solo deja una línea en el registro.

use tauri::{AppHandle, Emitter};
use tauri_plugin_autostart::ManagerExt;

use crate::preferencias::{self, Preferencias};
use crate::session::AppState;

/// Evento que avisa a la interfaz de que las preferencias cambiaron por dentro.
pub const EVENTO_PREFERENCIAS: &str = "signal://preferencias";

/// Qué hacer con el inicio automático.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accion {
    SoloLog,
    Habilitar,
    Deshabilitar,
    Nada,
}

/// Simulado → `SoloLog`; quiere y no activo → `Habilitar`; no quiere y activo → `Deshabilitar`; si no, `Nada`.
pub fn decidir(simulado: bool, quiere: bool, activo: bool) -> Accion {
    match (simulado, quiere, activo) {
        (true, _, _) => Accion::SoloLog,
        (false, true, false) => Accion::Habilitar,
        (false, false, true) => Accion::Deshabilitar,
        _ => Accion::Nada,
    }
}

/// Aplica la preferencia con el plugin de inicio automático (en simulado no lo llama).
pub fn sincronizar(app: &AppHandle, simulado: bool, quiere: bool) -> Result<(), String> {
    if simulado {
        eprintln!("inicio: modo simulado, no se registra el inicio con Windows (preferencia: {quiere})");
        return Ok(());
    }
    let gestor = app.autolaunch();
    let activo = gestor.is_enabled().map_err(|e| e.to_string())?;
    match decidir(simulado, quiere, activo) {
        Accion::Habilitar => gestor.enable().map_err(|e| e.to_string()),
        Accion::Deshabilitar => gestor.disable().map_err(|e| e.to_string()),
        Accion::SoloLog | Accion::Nada => Ok(()),
    }
}

/// Si la sincronización falló, la preferencia queda en `false`. Devuelve (preferencias, cambió).
pub fn corregir(mut p: Preferencias, resultado: &Result<(), String>) -> (Preferencias, bool) {
    if resultado.is_err() {
        p.iniciar_con_windows = false;
        (p, true)
    } else {
        (p, false)
    }
}

/// Sincroniza y corrige; si la preferencia cambió la guarda y avisa a la interfaz.
pub fn aplicar(app: &AppHandle, estado: &AppState, p: Preferencias) -> Preferencias {
    let resultado = sincronizar(app, estado.simulated, p.iniciar_con_windows);
    if let Err(e) = &resultado {
        eprintln!("inicio: no se pudo aplicar el inicio con Windows: {e}");
    }
    let (p, cambio) = corregir(p, &resultado);
    if !cambio {
        return p;
    }
    let p = match preferencias::guardar(&estado.config_dir, estado.simulated, &p) {
        Ok(guardada) => guardada,
        Err(e) => {
            eprintln!("inicio: no se pudo guardar la preferencia corregida: {e}");
            p
        }
    };
    if let Err(e) = app.emit(EVENTO_PREFERENCIAS, &p) {
        eprintln!("inicio: no se pudo emitir el cambio de preferencias: {e}");
    }
    p
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::preferencias::PrefAvisos;

    #[test]
    fn decidir_tabla() {
        for quiere in [false, true] {
            for activo in [false, true] {
                assert_eq!(decidir(true, quiere, activo), Accion::SoloLog);
            }
        }
        assert_eq!(decidir(false, true, false), Accion::Habilitar);
        assert_eq!(decidir(false, false, true), Accion::Deshabilitar);
        assert_eq!(decidir(false, true, true), Accion::Nada);
        assert_eq!(decidir(false, false, false), Accion::Nada);
    }

    #[test]
    fn corregir_ok_no_cambia_nada() {
        let p = Preferencias { iniciar_con_windows: true, ..Preferencias::default() };
        let (q, cambio) = corregir(p.clone(), &Ok(()));
        assert!(!cambio);
        assert_eq!(q, p);
    }

    #[test]
    fn corregir_error_apaga_solo_el_inicio() {
        let p = Preferencias {
            iniciar_con_windows: true,
            avisos: PrefAvisos { hora_cierre: "17:45".into(), ..PrefAvisos::default() },
            ..Preferencias::default()
        };
        let (q, cambio) = corregir(p.clone(), &Err("fallo".into()));
        assert!(cambio);
        assert!(!q.iniciar_con_windows);
        assert_eq!(q.avisos.hora_cierre, "17:45");
        assert_eq!(q.avisos, p.avisos);
        assert_eq!(q.minimizar_a_bandeja, p.minimizar_a_bandeja);
        assert_eq!(q.feriados_extra, p.feriados_extra);
    }
}
