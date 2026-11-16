//! Bucle que ejecuta el monitor: 30 segundos después de abrir y luego cada 15 minutos, nunca dos
//! ciclos a la vez. Cualquiera puede pedir un ciclo adelantado con `pedir_ciclo`.

use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::Notify;

use crate::escritorio;
use crate::monitor::{self, Memoria, Salida};
use crate::sesion::Estado;

/// Espera antes del primer ciclo.
pub const ESPERA_INICIAL: Duration = Duration::from_secs(30);
/// Tiempo entre ciclos.
pub const INTERVALO: Duration = Duration::from_secs(15 * 60);

/// Por qué terminó la espera.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motivo {
    Pedido,
    Tiempo,
}

/// Espera hasta `hasta` o hasta que alguien pida un ciclo.
pub async fn esperar(aviso: &Notify, hasta: Duration) -> Motivo {
    match tokio::time::timeout(hasta, aviso.notified()).await {
        Ok(()) => Motivo::Pedido,
        Err(_) => Motivo::Tiempo,
    }
}

/// Permite pedir un ciclo del monitor desde cualquier parte de la app.
pub struct Vigilante {
    aviso: Arc<Notify>,
}

impl Vigilante {
    pub fn nuevo() -> Self {
        Vigilante { aviso: Arc::new(Notify::new()) }
    }

    /// Pide un ciclo; varias peticiones seguidas se juntan en una.
    pub fn pedir(&self) {
        self.aviso.notify_one();
    }
}

/// Pide un ciclo si el vigilante ya está en el estado de Tauri.
pub fn pedir_ciclo(app: &AppHandle) {
    if let Some(vigilante) = app.try_state::<Vigilante>() {
        vigilante.pedir();
    }
}

/// Salida real del monitor: tooltip de la bandeja y notificaciones de Windows.
pub struct SalidaTauri(pub AppHandle);

impl Salida for SalidaTauri {
    fn tooltip(&self, texto: &str) {
        escritorio::fijar_tooltip(&self.0, texto);
    }

    fn notificar(&self, titulo: &str, cuerpo: &str) {
        if let Err(e) = self.0.notification().builder().title(titulo).body(cuerpo).show() {
            eprintln!("vigilancia: no se pudo mostrar la notificación: {e}");
        }
    }
}

/// Lanza el bucle del monitor en segundo plano.
pub fn iniciar(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let aviso = match app.try_state::<Vigilante>() {
            Some(v) => v.aviso.clone(),
            None => {
                eprintln!("vigilancia: no hay vigilante en el estado, el monitor no arranca");
                return;
            }
        };
        let estado = app.state::<Estado>();
        let salida = SalidaTauri(app.clone());
        let mut memoria = Memoria::default();
        let mut espera = ESPERA_INICIAL;
        loop {
            esperar(&aviso, espera).await;
            let r = monitor::ciclo(&estado, &salida, monitor::ahora_ecuador(), &mut memoria).await;
            eprintln!("vigilancia: ciclo terminado: {r:?}");
            espera = INTERVALO;
        }
    });
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[tokio::test]
    async fn sin_pedir_termina_por_tiempo() {
        let aviso = Notify::new();
        assert_eq!(esperar(&aviso, Duration::from_millis(50)).await, Motivo::Tiempo);
    }

    #[tokio::test]
    async fn un_pedido_previo_se_atiende_al_instante() {
        let aviso = Notify::new();
        aviso.notify_one();
        assert_eq!(esperar(&aviso, Duration::from_secs(5)).await, Motivo::Pedido);
    }

    #[tokio::test]
    async fn dos_pedidos_seguidos_se_juntan_en_uno() {
        let aviso = Notify::new();
        aviso.notify_one();
        aviso.notify_one();
        assert_eq!(esperar(&aviso, Duration::from_secs(5)).await, Motivo::Pedido);
        assert_eq!(esperar(&aviso, Duration::from_millis(50)).await, Motivo::Tiempo);
    }

    #[test]
    fn tiempos_fijados() {
        assert_eq!(INTERVALO.as_secs(), 900);
        assert_eq!(ESPERA_INICIAL.as_secs(), 30);
    }

    #[tokio::test]
    async fn el_vigilante_despierta_la_espera() {
        let v = Vigilante::nuevo();
        v.pedir();
        assert_eq!(esperar(&v.aviso, Duration::from_secs(5)).await, Motivo::Pedido);
    }
}
