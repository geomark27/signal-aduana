//! Bucle que ejecuta el monitor: 30 segundos después de abrir y luego cada 15 minutos, nunca dos
//! ciclos a la vez. Cualquiera puede pedir un ciclo adelantado con `pedir_ciclo`.

use std::any::Any;
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use std::time::Duration;

use futures::FutureExt;
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
        bucle(&estado, &salida, &aviso, ESPERA_INICIAL, INTERVALO, None).await;
    });
}

const MAX_MENSAJE_PANICO: usize = 200;

fn mensaje_de_panico(payload: &(dyn Any + Send)) -> String {
    let texto = if let Some(s) = payload.downcast_ref::<&str>() {
        s
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.as_str()
    } else {
        "sin mensaje"
    };
    texto.chars().take(MAX_MENSAJE_PANICO).collect()
}

/// Ejecuta `f`; si entra en pánico devuelve `Err` con el mensaje del pánico.
pub async fn ciclo_protegido<F: Future>(f: F) -> Result<F::Output, String> {
    AssertUnwindSafe(f).catch_unwind().await.map_err(|p| mensaje_de_panico(p.as_ref()))
}

/// Bucle del monitor: espera, hace un ciclo y repite. Un pánico del ciclo se registra, se
/// muestra en el tooltip y no detiene el bucle. `max_ciclos` es `None` en producción.
pub async fn bucle(
    estado: &Estado,
    salida: &dyn Salida,
    aviso: &Notify,
    espera_inicial: Duration,
    intervalo: Duration,
    max_ciclos: Option<usize>,
) {
    let mut memoria = Memoria::default();
    let mut espera = espera_inicial;
    let mut hechos = 0usize;
    while max_ciclos.map_or(true, |max| hechos < max) {
        esperar(aviso, espera).await;
        match ciclo_protegido(monitor::ciclo(estado, salida, monitor::ahora_ecuador(), &mut memoria)).await {
            Ok(r) => eprintln!("vigilancia: ciclo terminado: {r:?}"),
            Err(mensaje) => {
                eprintln!("vigilancia: el ciclo entró en pánico: {mensaje}");
                let texto = monitor::texto_error_interno(estado.simulado);
                if catch_unwind(AssertUnwindSafe(|| salida.tooltip(&texto))).is_err() {
                    eprintln!("vigilancia: no se pudo fijar el tooltip de error interno");
                }
            }
        }
        hechos += 1;
        espera = intervalo;
    }
}

#[cfg(test)]
mod pruebas {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    use super::*;
    use crate::almacen::AlmacenMemoria;

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

    #[tokio::test]
    async fn ciclo_protegido_devuelve_el_valor_o_el_mensaje_del_panico() {
        assert_eq!(ciclo_protegido(async { 7 }).await, Ok(7));
        let r = ciclo_protegido(async {
            if true {
                panic!("boom");
            }
            0
        })
        .await;
        assert!(r.unwrap_err().contains("boom"));
    }

    #[tokio::test]
    async fn el_mensaje_del_panico_se_recorta() {
        let largo = "x".repeat(500);
        let r = ciclo_protegido(async move {
            if true {
                panic!("{largo}");
            }
        })
        .await;
        assert_eq!(r.unwrap_err().chars().count(), MAX_MENSAJE_PANICO);
    }

    struct SalidaQueFallaUnaVez {
        llamadas: AtomicUsize,
        tooltips: Mutex<Vec<String>>,
    }

    impl Salida for SalidaQueFallaUnaVez {
        fn tooltip(&self, texto: &str) {
            if self.llamadas.fetch_add(1, Ordering::SeqCst) == 0 {
                panic!("fallo provocado en la prueba");
            }
            self.tooltips.lock().unwrap().push(texto.to_string());
        }
        fn notificar(&self, _titulo: &str, _cuerpo: &str) {}
    }

    #[tokio::test]
    async fn el_bucle_sobrevive_a_un_panico_del_ciclo() {
        let dir = tempfile::tempdir().unwrap();
        let estado = Estado::nuevo(
            true,
            dir.path().to_path_buf(),
            dir.path().join("datos"),
            Box::new(AlmacenMemoria::default()),
        );
        let salida = SalidaQueFallaUnaVez { llamadas: AtomicUsize::new(0), tooltips: Mutex::new(Vec::new()) };
        let aviso = Notify::new();
        let ms = Duration::from_millis(5);
        tokio::time::timeout(Duration::from_secs(5), bucle(&estado, &salida, &aviso, ms, ms, Some(2)))
            .await
            .expect("el bucle no terminó");
        let tooltips = salida.tooltips.lock().unwrap().clone();
        assert_eq!(tooltips, vec![monitor::texto_error_interno(true), monitor::texto_sin_sesion(true)]);
        assert!(tooltips[0].ends_with(" (simulado)"));
    }

    struct SalidaQueSiempreFalla;

    impl Salida for SalidaQueSiempreFalla {
        fn tooltip(&self, _texto: &str) {
            panic!("tooltip roto");
        }
        fn notificar(&self, _titulo: &str, _cuerpo: &str) {}
    }

    #[tokio::test]
    async fn un_segundo_panico_al_fijar_el_tooltip_tampoco_termina_el_bucle() {
        let dir = tempfile::tempdir().unwrap();
        let estado = Estado::nuevo(
            true,
            dir.path().to_path_buf(),
            dir.path().join("datos"),
            Box::new(AlmacenMemoria::default()),
        );
        let aviso = Notify::new();
        let ms = Duration::from_millis(5);
        tokio::time::timeout(Duration::from_secs(5), bucle(&estado, &SalidaQueSiempreFalla, &aviso, ms, ms, Some(3)))
            .await
            .expect("el bucle no terminó");
    }
}
