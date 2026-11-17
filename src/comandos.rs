//! Comandos de Tauri (Contrato B). Cada uno es una capa fina sobre `sesion` y `jira`.
//! Si un comando de datos recibe `NoAutorizado`, la sesión en memoria se descarta.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use serde_json::Value;
use tauri::{AppHandle, State};

use crate::cache::{self, Instantanea};
use crate::domain::{self, View, ViewInput};
use crate::error::{ErrorApp, Resultado};
use crate::escritorio;
use crate::inicio;
use crate::jira::modelo::Tarea;
use crate::jira::{self, ResultadoHoras};
use crate::monitor;
use crate::preferencias::{self, Preferencias};
use crate::sesion::{self, Estado, InfoSesion, Sesion};
use crate::vigilancia;

/// Ejecuta una operación con la sesión activa y limpia la sesión si Jira rechaza el token.
async fn con_sesion<T, F, Fut>(estado: &Estado, f: F) -> Resultado<T>
where
    F: FnOnce(Arc<Sesion>) -> Fut,
    Fut: Future<Output = Resultado<T>>,
{
    let s = estado.actual().await?;
    let r = f(s).await;
    if matches!(r, Err(ErrorApp::NoAutorizado)) {
        estado.limpiar().await;
    }
    r
}

#[tauri::command]
pub async fn restaurar_sesion(app: AppHandle, estado: State<'_, Estado>) -> Resultado<InfoSesion> {
    let info = sesion::restaurar(&estado).await;
    if info.conectado {
        vigilancia::pedir_ciclo(&app);
    }
    Ok(info)
}

#[tauri::command]
pub async fn iniciar_sesion(
    app: AppHandle,
    estado: State<'_, Estado>,
    sitio: String,
    correo: String,
    token: String,
) -> Resultado<InfoSesion> {
    let info = sesion::iniciar(&estado, &sitio, &correo, &token).await?;
    vigilancia::pedir_ciclo(&app);
    Ok(info)
}

#[tauri::command]
pub async fn cerrar_sesion(app: AppHandle, estado: State<'_, Estado>) -> Resultado<InfoSesion> {
    let info = sesion::cerrar(&estado).await;
    escritorio::fijar_tooltip(&app, &monitor::texto_sin_sesion(estado.simulado));
    Ok(info)
}

#[tauri::command]
pub async fn cargar_tareas(estado: State<'_, Estado>) -> Resultado<Vec<Tarea>> {
    con_sesion(&estado, |s| async move {
        jira::cargar_tareas(&s.fuente, Some(&s.usuario.account_id)).await
    })
    .await
}

#[tauri::command]
pub async fn claves_con_horas(estado: State<'_, Estado>, desde: String) -> Resultado<Vec<String>> {
    con_sesion(&estado, |s| async move { jira::claves_con_horas(&s.fuente, &desde).await }).await
}

#[tauri::command]
pub async fn claves_con_pr(estado: State<'_, Estado>) -> Resultado<Vec<String>> {
    con_sesion(&estado, |s| async move { jira::claves_con_pr(&s.fuente).await }).await
}

#[tauri::command]
pub async fn leer_horas(
    estado: State<'_, Estado>,
    claves: Vec<String>,
    desde: i64,
) -> Resultado<ResultadoHoras> {
    con_sesion(&estado, |s| async move {
        jira::leer_horas(&s.fuente, &claves, desde, Some(&s.usuario.account_id)).await
    })
    .await
}

#[tauri::command]
pub async fn buscar_documentos(
    estado: State<'_, Estado>,
    claves: Vec<String>,
) -> Resultado<HashMap<String, String>> {
    con_sesion(&estado, |s| async move { jira::buscar_documentos(&s.fuente, &claves).await }).await
}

/// Última carga guardada de la cuenta activa, o `null`.
#[tauri::command]
pub async fn leer_cache(estado: State<'_, Estado>) -> Resultado<Option<Instantanea>> {
    leer_cache_de(&estado).await
}

#[tauri::command]
pub async fn guardar_cache(estado: State<'_, Estado>, datos: Value) -> Resultado<()> {
    guardar_cache_de(&estado, datos).await
}

/// Calcula todo el tablero de una semana a partir de las tareas recibidas. No requiere sesión.
#[tauri::command]
pub fn calcular_vista(entrada: ViewInput) -> Resultado<View> {
    let hoy_ecuador = crate::jira::simulado::hoy_ecuador().format("%Y-%m-%d").to_string();
    domain::compute_view(entrada, &hoy_ecuador)
}

/// Preferencias del equipo (valores por defecto si no hay archivo o está dañado). No requiere sesión.
#[tauri::command]
pub fn leer_preferencias(estado: State<'_, Estado>) -> Preferencias {
    preferencias::cargar(&estado.dir_config, estado.simulado)
}

/// Normaliza y guarda las preferencias; devuelve las que quedaron guardadas. No requiere sesión.
#[tauri::command]
pub fn guardar_preferencias(
    app: AppHandle,
    estado: State<'_, Estado>,
    preferencias: Preferencias,
) -> Resultado<Preferencias> {
    let anteriores = preferencias::cargar(&estado.dir_config, estado.simulado);
    let p = preferencias::guardar(&estado.dir_config, estado.simulado, &preferencias)?;
    let guardadas = inicio::aplicar(&app, &estado, p);
    if anteriores.afecta_al_monitor(&guardadas) {
        vigilancia::pedir_ciclo(&app);
    }
    Ok(guardadas)
}

async fn leer_cache_de(estado: &Estado) -> Resultado<Option<Instantanea>> {
    let s = estado.actual().await?;
    Ok(cache::leer(&estado.dir_datos, estado.simulado, &s.usuario.account_id, &s.sitio.como_texto()))
}

async fn guardar_cache_de(estado: &Estado, datos: Value) -> Resultado<()> {
    let s = estado.actual().await?;
    cache::guardar(&estado.dir_datos, estado.simulado, &s.usuario.account_id, &s.sitio.como_texto(), datos)
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::almacen::AlmacenMemoria;

    #[tokio::test]
    async fn sin_sesion_y_no_autorizado_limpia() {
        let dir = tempfile::tempdir().unwrap();
        let e = Estado::nuevo(true, dir.path().to_path_buf(), dir.path().join("datos"), Box::new(AlmacenMemoria::default()));
        let r = con_sesion(&e, |_| async { Ok(()) }).await;
        assert!(matches!(r, Err(ErrorApp::SinSesion)));

        sesion::iniciar(&e, "https://x.atlassian.net", "a@b.com", "t").await.unwrap();
        let r: Resultado<()> = con_sesion(&e, |_| async { Err(ErrorApp::NoAutorizado) }).await;
        assert!(matches!(r, Err(ErrorApp::NoAutorizado)));
        assert!(matches!(e.actual().await, Err(ErrorApp::SinSesion)));
    }

    #[tokio::test]
    async fn cache_requiere_sesion_y_hace_ida_y_vuelta() {
        let dir = tempfile::tempdir().unwrap();
        let e = Estado::nuevo(true, dir.path().to_path_buf(), dir.path().join("datos"), Box::new(AlmacenMemoria::default()));
        assert!(matches!(leer_cache_de(&e).await, Err(ErrorApp::SinSesion)));
        assert!(matches!(guardar_cache_de(&e, serde_json::json!(1)).await, Err(ErrorApp::SinSesion)));

        sesion::iniciar(&e, "https://x.atlassian.net", "a@b.com", "t").await.unwrap();
        assert!(leer_cache_de(&e).await.unwrap().is_none());
        guardar_cache_de(&e, serde_json::json!({ "formato": 1 })).await.unwrap();
        let i = leer_cache_de(&e).await.unwrap().unwrap();
        assert_eq!(i.datos, serde_json::json!({ "formato": 1 }));
    }
}
